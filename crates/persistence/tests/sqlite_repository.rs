use application::ports::SaveRepository;
use application::{GameState, SaveError};
use domain::breeding::{Egg, IncubationState};
use domain::combat::{BattleState, Combatant, Element};
use domain::genetics::{Ancestry, GeneticProfile, Parentage};
use domain::identifiers::{BattleId, EggId, ItemId, PetId, SaveSlotId, SpeciesId};
use domain::pet::{Personality, Pet, PetName};
use domain::rng::WorldSeed;
use domain::time::{GameDuration, Tick, UnixSeconds};
use fs4::FileExt;
use persistence::SqliteSaveRepository;
use rusqlite::Connection;

fn slot() -> SaveSlotId {
    SaveSlotId::derive(WorldSeed(9), Tick::ZERO, 1)
}
fn state() -> GameState {
    let seed = WorldSeed(42);
    let mut state = GameState::new(seed, UnixSeconds(1_000));
    let first = PetId::derive(seed, Tick::ZERO, 1);
    let second = PetId::derive(seed, Tick::ZERO, 2);
    for (index, id) in [first, second].into_iter().enumerate() {
        let mut pet = Pet::hatch(
            id,
            SpeciesId::new("emberling").unwrap(),
            GeneticProfile::new(),
            Personality::neutral(),
            Parentage::Founder,
            Ancestry::founder(),
            Tick::ZERO,
        );
        pet.rename(PetName::new(format!("Pet {}", index + 1)).unwrap());
        state.simulation.insert_pet(pet);
    }
    let egg_id = EggId::derive(seed, Tick::ZERO, 3);
    state.eggs.insert(
        egg_id,
        Egg {
            id: egg_id,
            parents: [first, second],
            species: SpeciesId::new("emberling").unwrap(),
            genetics: GeneticProfile::new(),
            ancestry: Ancestry::founder(),
            incubation: IncubationState {
                laid_at: Tick::ZERO,
                required: GameDuration::from_days(2),
            },
            mutations: Vec::new(),
        },
    );
    let battle_id = BattleId::derive(seed, Tick::ZERO, 4);
    state.active_battles.insert(
        battle_id,
        BattleState {
            id: battle_id,
            combatants: [
                Combatant::from_stats(first, Element::Ember, domain::pet::Stats::default()),
                Combatant::from_stats(second, Element::Ember, domain::pet::Stats::default()),
            ],
            turn: 0,
        },
    );
    state
        .inventory
        .add(ItemId::new("apple_slice").unwrap(), 3)
        .unwrap();
    state
}

#[test]
fn save_load_is_exact_and_derived_rows_are_rebuilt() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("save.db");
    let repo = SqliteSaveRepository::new(&path).unwrap();
    let expected = state();
    repo.save(slot(), &expected).unwrap();
    assert_eq!(repo.load(slot()).unwrap(), expected);
    assert_eq!(repo.list_slots().unwrap().len(), 1);
    assert!(std::path::PathBuf::from(format!("{}.daily", path.display())).is_file());
    let connection = Connection::open(path).unwrap();
    let pets: i64 = connection
        .query_row("SELECT COUNT(*) FROM pets", [], |row| row.get(0))
        .unwrap();
    assert_eq!(pets, 2);
    let names: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM pets WHERE name IN ('Pet 1', 'Pet 2')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(names, 2);
    let items: i64 = connection
        .query_row("SELECT COUNT(*) FROM inventory", [], |row| row.get(0))
        .unwrap();
    assert_eq!(items, 1);
    let eggs: i64 = connection
        .query_row("SELECT COUNT(*) FROM eggs", [], |row| row.get(0))
        .unwrap();
    assert_eq!(eggs, 1);
    let battles: i64 = connection
        .query_row("SELECT COUNT(*) FROM active_battles", [], |row| row.get(0))
        .unwrap();
    assert_eq!(battles, 1);
}

#[test]
fn opening_a_schema_three_database_adds_the_pet_name_read_model() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("legacy.db");
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE save_slots(slot BLOB PRIMARY KEY, schema_version INTEGER NOT NULL, updated_at INTEGER NOT NULL);
             CREATE TABLE pets(slot BLOB NOT NULL REFERENCES save_slots(slot), pet_id TEXT NOT NULL, species TEXT NOT NULL, stage TEXT NOT NULL, PRIMARY KEY(slot, pet_id));",
        )
        .unwrap();
    drop(connection);

    SqliteSaveRepository::new(&path).unwrap();

    let connection = Connection::open(path).unwrap();
    let columns = connection
        .prepare("PRAGMA table_info(pets)")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(columns.iter().any(|column| column == "name"));
}

#[test]
fn invalid_save_cannot_replace_the_previous_snapshot() {
    let temp = tempfile::tempdir().unwrap();
    let repo = SqliteSaveRepository::new(temp.path().join("save.db")).unwrap();
    let expected = state();
    repo.save(slot(), &expected).unwrap();
    let mut invalid = expected.clone();
    invalid.simulation.tick = Tick(1);
    assert!(matches!(
        repo.save(slot(), &invalid),
        Err(SaveError::InvalidState(_))
    ));
    assert_eq!(repo.load(slot()).unwrap(), expected);
}

#[test]
fn corrupt_primary_recovers_from_latest_backup() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("save.db");
    let repo = SqliteSaveRepository::new(&path).unwrap();
    let expected = state();
    repo.save(slot(), &expected).unwrap();
    Connection::open(&path)
        .unwrap()
        .execute("UPDATE game_snapshots SET envelope=randomblob(32)", [])
        .unwrap();
    assert_eq!(repo.load(slot()).unwrap(), expected);
}

#[test]
fn concurrent_writer_gets_locked_error() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("save.db");
    let repo = SqliteSaveRepository::new(&path).unwrap();
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path.with_extension("lock"))
        .unwrap();
    FileExt::lock(&lock).unwrap();
    assert!(matches!(
        repo.save(slot(), &state()),
        Err(SaveError::Locked)
    ));
    FileExt::unlock(&lock).unwrap();
}

#[test]
fn slots_can_be_deleted_without_orphans() {
    let temp = tempfile::tempdir().unwrap();
    let repo = SqliteSaveRepository::new(temp.path().join("save.db")).unwrap();
    repo.save(slot(), &state()).unwrap();
    repo.delete(slot()).unwrap();
    assert!(matches!(repo.load(slot()), Err(SaveError::NotFound)));
    assert!(repo.list_slots().unwrap().is_empty());
}
