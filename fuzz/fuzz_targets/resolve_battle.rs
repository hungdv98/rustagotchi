#![no_main]

use std::sync::OnceLock;

use domain::combat::{BattleAction, BattleState, Combatant, Element};
use domain::identifiers::{BattleId, MoveId, PetId};
use domain::pet::Stats;
use domain::rng::WorldSeed;
use domain::time::Tick;
use libfuzzer_sys::fuzz_target;

fn registry() -> &'static content::ContentRegistry {
    static CONTENT: OnceLock<content::ContentRegistry> = OnceLock::new();
    CONTENT.get_or_init(|| {
        content::ContentLoader::load(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets"),
        )
        .expect("production content")
    })
}

fuzz_target!(|data: &[u8]| {
    if data.len() < 9 {
        return;
    }
    let seed = WorldSeed(u64::from_le_bytes(data[..8].try_into().unwrap()));
    let first = PetId::derive(seed, Tick::ZERO, 1);
    let second = PetId::derive(seed, Tick::ZERO, 2);
    let state = BattleState {
        id: BattleId::derive(seed, Tick::ZERO, 3),
        combatants: [
            Combatant::from_stats(first, Element::Ember, Stats::uniform(10)),
            Combatant::from_stats(second, Element::Ember, Stats::uniform(10)),
        ],
        turn: 0,
    };
    let move_ids = ["tackle", "warm_nuzzle", "static_puff"];
    let actions = data[8..]
        .chunks(2)
        .take(20)
        .map(|choice| {
            [first, second].map(|actor| BattleAction {
                actor,
                move_id: MoveId::new(move_ids[usize::from(choice[0]) % move_ids.len()]).unwrap(),
            })
        })
        .collect::<Vec<_>>();
    let _ = simulation::combat::resolve_battle(seed, state, &actions, registry());
});
