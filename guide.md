# Rustagotchi: Run and Play Guide

This guide takes you from a fresh checkout to running the desktop game, playing
the current vertical slice, working with saves, and using the developer tools.
Run all commands from the repository root.

## 1. Install prerequisites

You need:

- [Git](https://git-scm.com/)
- [Rustup](https://rustup.rs/)
- A C/C++ build toolchain
  - Windows: Visual Studio Build Tools with **Desktop development with C++**
  - Linux: a C compiler and the development packages required by X11/winit
  - macOS: Xcode Command Line Tools (`xcode-select --install`)

The repository pins Rust `1.97.1` in `rust-toolchain.toml`. Rustup selects and
installs it automatically when you run a Cargo command in this directory.

## 2. Get the project

```bash
git clone <repository-url>
cd rustagotchi
```

If you already have the project, open a terminal in its root. You should see
`Cargo.toml`, `assets/`, `crates/`, and `scripts/` there.

## 3. Run the one-time developer setup

On Windows PowerShell:

```powershell
.\scripts\setup.ps1
```

On Linux or macOS:

```bash
./scripts/setup.sh
```

The setup installs the pinned toolchain and the Cargo tools used by the full
quality gate. It also copies `.env.example` to `.env` when `.env` is absent.

Fuzzing is optional. To install its nightly toolchain and `cargo-fuzz`:

```powershell
.\scripts\setup.ps1 -IncludeNightly
```

```bash
./scripts/setup.sh --with-nightly
```

## 4. Verify the checkout

Use the fast gate during normal development:

```powershell
.\scripts\check.ps1 -Fast
```

```bash
./scripts/check.sh --fast
```

It checks formatting, strict Clippy lints, dependency direction, unit and
integration tests, and doctests. Before a release or pull request, run the full
gate, which also performs supply-chain checks, content validation, and a release
build:

```powershell
.\scripts\check.ps1
```

```bash
./scripts/check.sh
```

The first Bevy build can take several minutes. Later builds reuse Cargo's cache.

## 5. Start the desktop game

Run:

```bash
cargo run -p rustagotchi-desktop
```

For a faster packaged-style executable after the longer optimized build:

```bash
cargo run --release -p rustagotchi-desktop
```

Keep the repository root as the working directory so the client finds
`assets/manifest.ron`. Only one Rustagotchi process can use a profile at a time.

### Controls

- Mouse: point and click a button.
- Keyboard: `Tab` selects the next button.
- Keyboard: `Shift+Tab` selects the previous button.
- Keyboard: `Enter` or `Space` activates the focused button.
- Close the window normally to save before exit.

The main menu has compact **Sound: On/Off** and **Motion: Full/Reduced**
controls. Their current state is also summarized above the buttons. Reduced motion
disables event-driven screen pulses while preserving text feedback, and muting
sound does not affect native critical-need notifications.

Select **Reset game** to open a destructive-action confirmation screen. A
confirmed reset deletes the active save slot and clears every pet, egg, item,
coin, cooldown, and battle before returning to adoption. **Cancel** leaves the
world unchanged.

A keyboard-focused button has a gold outline. Button roles and labels are also
published to the operating system's accessibility services.

The window can be resized down to a supported minimum of 360×540. Content wraps
inside a centered layout, and command successes or failures appear in a status
message on every screen. Errors remain visible until a later user command
succeeds; background clock updates do not dismiss them.

## 6. Play the current game flow

### Create your first pet

1. Start the desktop client.
2. Select **Continue** on the main menu.
3. Enter a pet name of 1-20 characters.
4. Select **Emberling**, **Stormling**, or **Aquafin**.
5. The game creates the world and opens **Home**. The name persists across saves.

The adoption screen and Home summary display **Adopted N/capacity**. A new world
starts with two founder slots. Use **Adopt** on Home to return to this screen and
buy permanent slot upgrades with battle-earned coins. Prices are 100, 200, 400,
800, 1,600, and 3,200 coins, raising the limit one slot at a time to eight.
Bred and hatched offspring do not consume founder adoption slots.

The Home screen displays a responsive named gallery of up to eight pets using
distinct Emberling, Stormling, and Aquafin artwork as pets evolve. Each
pet's idle and activity motion comes from deterministic game state, while care,
danger, breeding, hatching, and battle events can briefly override the gallery
with visual reactions. Enabling reduced motion keeps every portrait stationary
without hiding status text.

### Inspect and care for a pet

1. From **Home**, select **Status**.
2. Review the selected pet's species, life stage, mood, battle level, XP toward
   the next level, and labeled need
   meters beside its live species portrait. Fullness is shown instead of
   "hunger" so a higher percentage always means a healthier state.
3. When fullness is below 100%, select the **Feed apple** care card. Its second
   line shows how many slices are available. At 100%, the disabled card reads
   **Pet is full — No food needed**.
4. Select **Next pet** to change the active pet when more than one exists. With
   one pet, the disabled control reads **Only one pet**.
5. Select **Inventory** to inspect all item quantities.

The first founder receives five apple slices. Feeding consumes one slice only
when the care command succeeds.

### Adopt founders and unlock more slots

1. From **Home**, open **Breed** or **Battle**.
2. If the world has only one pet, select **Adopt companion**.
3. Enter a name and select **Emberling**, **Stormling**, or **Aquafin**.
4. The game returns to **Home**.

After filling the initial two founder slots, win two battles to earn the 100
coins needed for the first upgrade. From Home, select **Adopt**, review the next
slot price and your balance, then select **Unlock slot 3 of 8**. Each purchase
is saved with the world. A failed purchase never removes coins.

### Release a pet

1. From **Home**, select **Status** and cycle to the pet you intend to release.
2. Select **Release**.
3. Review the permanent-action warning, then select **Confirm release** or
   **Keep pet** to cancel.

A pet cannot be released during an active battle or while it is the parent of
an unhatched egg. Releasing an adopted founder frees one of the unlocked adoption
slots. Release is permanent, so save backups are the only recovery route after
the game is saved.

### Breed and hatch an egg

The selected parents must both be healthy adults at least seven in-game days
old. Until then, **Parents need 7 days** is disabled; attempting the same
operation through another input path produces a friendly notice rather than an
application error.

1. Let both founders reach the adult life stage and keep them healthy.
2. From **Home**, open **Breeding**.
3. Use **Next parent A** and **Next parent B** to choose distinct parents.
4. Select **Breed selected pair**.
5. Return after incubation and select **Hatch egg** when it becomes available.

One simulation tick is one game minute. Pets become adults at seven in-game
days, and normal egg incubation takes two in-game days. The desktop advances
from wall-clock time while running and applies the same deterministic catch-up
when relaunched after an absence.

If breeding is refused, the screen reports the reason. Both parents must be
living, healthy adults, old enough to breed, distinct, compatible, and outside
their breeding cooldown.

### Run a battle

1. From **Home**, select **Battle**.
2. Use **Next A** and **Next B** to select two living pets.
3. Review each fighter's named move pool with power, accuracy, and energy cost.
4. Select **Start battle**.
5. Choose **Power** for maximum move power, **Precision** for accuracy, or
   **Efficiency** for damage per energy. The selected deterministic tactic
   drives both fighters' move choices.
6. Review the dedicated victory card. It names the winner, shows the 50-coin and
   100-XP rewards, updated wallet, current level, XP progress, and any level-up.

The winner receives XP and can advance through levels 1-99. Level 1 requires
100 XP, so the first victory visibly reaches level 2. Coins are shared by the
world and fund permanent adoption-slot upgrades.

### Save, quit, and apply offline progress

1. Close the desktop window normally. The application queues a save-on-exit.
2. Start it again with `cargo run -p rustagotchi-desktop`.
3. The latest save is loaded automatically.
4. Time elapsed while the app was closed is applied before the main menu opens.
5. Select **Save slots** to inspect saved slots or load the next available slot.

Critical needs and pet deaths can produce best-effort native notifications,
including events generated during offline catch-up. Notification delivery still
depends on operating-system permissions and services.

## 7. Locate and back up saves

The desktop database is named `rustagotchi.sqlite3`. Default locations are:

| Platform | Save directory |
| --- | --- |
| Windows | `%LOCALAPPDATA%\Rustagotchi\Rustagotchi\data\saves` |
| Linux | `${XDG_DATA_HOME:-$HOME/.local/share}/rustagotchi/saves` |
| macOS | `$HOME/Library/Application Support/com.Rustagotchi.Rustagotchi/saves` |

Close the game before copying or moving the database. On Windows PowerShell:

```powershell
$saveDir = Join-Path $env:LOCALAPPDATA 'Rustagotchi\Rustagotchi\data\saves'
Get-ChildItem $saveDir
Copy-Item (Join-Path $saveDir 'rustagotchi.sqlite3') (Join-Path $saveDir 'rustagotchi.sqlite3.backup')
```

On Linux:

```bash
save_dir="${XDG_DATA_HOME:-$HOME/.local/share}/rustagotchi/saves"
ls -la "$save_dir"
cp "$save_dir/rustagotchi.sqlite3" "$save_dir/rustagotchi.sqlite3.backup"
```

To start a fresh profile, first close the game and make a backup, then rename
`rustagotchi.sqlite3`. The next launch creates a new world. Renaming keeps the
old profile recoverable.

## 8. Configure a run

The desktop currently reads these process environment variables:

- `RUST_LOG`: tracing filter, such as `info` or
  `info,rustagotchi_simulation=debug`.
- `RUSTAGOTCHI_ASSETS`: path to a directory containing `manifest.ron`.

PowerShell example:

```powershell
$env:RUST_LOG = 'info,rustagotchi_simulation=debug'
$env:RUSTAGOTCHI_ASSETS = (Resolve-Path .\assets).Path
cargo run -p rustagotchi-desktop
```

Bash example:

```bash
export RUST_LOG='info,rustagotchi_simulation=debug'
export RUSTAGOTCHI_ASSETS="$PWD/assets"
cargo run -p rustagotchi-desktop
```

The setup script creates `.env` as a development template, but the desktop does
not currently load that file automatically. Export variables in the launching
shell as shown above.

## 9. Use the headless tools

Show all commands:

```bash
cargo run -p simulation-cli -- --help
```

Validate the production content pack:

```bash
cargo run -p simulation-cli -- validate-content --assets assets
```

Run a deterministic one-day simulation. One day is 1,440 ticks:

```bash
cargo run -p simulation-cli -- simulate --seed 42 --ticks 1440 --assets assets
```

Run the committed vertical-slice scenario and save its result:

```bash
cargo run -p simulation-cli -- simulate \
  --script tests/scripts/vertical_slice.ron \
  --seed 42 \
  --assets assets \
  --save rustagotchi-demo.sqlite3
```

PowerShell accepts the same command on one line:

```powershell
cargo run -p simulation-cli -- simulate --script tests/scripts/vertical_slice.ron --seed 42 --assets assets --save rustagotchi-demo.sqlite3
```

Inspect a SQLite save:

```bash
cargo run -p simulation-cli -- inspect-save rustagotchi-demo.sqlite3
cargo run -p simulation-cli -- inspect-save rustagotchi-demo.sqlite3 --json
```

Run smaller local balance checks:

```bash
cargo run --release -p simulation-cli -- benchmark --offspring 1000 --battles 1000
```

Use `cargo run -p simulation-cli -- <command> --help` for every command's full
option list.

## 10. Troubleshooting

### `cannot find assets`

Run from the repository root or set `RUSTAGOTCHI_ASSETS` to the absolute path of
the `assets` directory. That directory must contain `manifest.ron`.

### `another Rustagotchi process is already using this profile`

Close the other game window. If the previous process crashed, confirm no
`rustagotchi-desktop` process remains, then launch again. The process lock is
released when the process exits.

### The first build appears slow

Bevy, SQLite, zstd, and the graphics stack have substantial initial compile
costs. Let the first build finish; subsequent builds are incremental.

### Linker or native-library errors

Confirm the platform build prerequisites from section 1 are installed. On
Windows, use a terminal where the MSVC Build Tools are available. On Linux,
install your distribution's C compiler and X11 development packages.

### No native notifications appear

Check the operating system's notification permissions and do-not-disturb mode.
Notifications are best-effort and do not affect simulation or saves.

### A save cannot be opened

Keep the original database intact and run `inspect-save` against a copy. The
persistence layer validates checksums and can recover a corrupt primary payload
from its latest valid backup when one is available.

## 11. Recommended development loop

1. Make a focused change.
2. Run `cargo fmt --all`.
3. Run the relevant package tests, for example
   `cargo test -p rustagotchi-presentation`.
4. Run the fast repository gate.
5. Run the full gate before submitting or releasing the change.

The headless desktop smoke flows can also be run directly. They use the real
Bevy presentation plugin and temporary SQLite databases without opening a
window, covering preference controls, care/battle, and
breeding/incubation/hatching:

```bash
cargo test -p rustagotchi-presentation --test desktop_smoke
```

The same test binary constructs eight real portrait nodes and runs 1,000
headless frames after warm-up. It fails if average presentation CPU time exceeds
the 16.67 ms frame budget. This guards CPU regressions in CI; the native
verifier below covers the separate GPU/window path.

Run the native rendered-window check with:

```bash
cargo run -p rustagotchi-desktop --bin render-verify
```

It uses a deterministic disposable eight-pet world and never opens the user's
save repository. The window closes after capturing and validating a 1100×720
frame. By default the artifact is written to
`target/render-verification/rustagotchi-home.png`; set
`RUSTAGOTCHI_RENDER_OUTPUT` to choose another file. CI runs this command under
the real Windows window stack and Linux Xvfb/Mesa stack, then uploads both PNGs.

To capture another screen, set `RUSTAGOTCHI_RENDER_SCREEN` to `pet-status`,
`pet-creation`, `battle`, or `battle-result`. In PowerShell:

```powershell
$env:RUSTAGOTCHI_RENDER_SCREEN = "pet-status"
cargo run -p rustagotchi-desktop --bin render-verify
```

Architecture and implementation details live in
[`docs/implementation-plan.md`](docs/implementation-plan.md). Contributor and
security policies live in [`CONTRIBUTING.md`](CONTRIBUTING.md) and
[`SECURITY.md`](SECURITY.md).
