//! Fixed-order simulation systems.

use domain::events::DeathCause;
use domain::pet::health::Severity;
use domain::pet::{Activity, Experience, LifeStage, NeedKind, NeedValue};
use domain::rng::{RngStream, rng_for};
use domain::time::{GameDuration, TICKS_PER_DAY};
use domain::units::Milli;

use crate::scheduler::{SimSystem, SystemId, TickContext};

macro_rules! system {
    ($name:ident, $id:ident, $body:expr) => {
        #[derive(Debug)]
        pub struct $name;
        impl SimSystem for $name {
            const ID: SystemId = SystemId::$id;
            fn run(ctx: &mut TickContext<'_>) {
                $body(ctx)
            }
        }
    };
}

system!(NeedsDecay, NeedsDecay, |ctx: &mut TickContext<'_>| {
    for pet in ctx.state.pets.values_mut().filter(|pet| pet.is_alive()) {
        for kind in NeedKind::ALL {
            let next = pet
                .needs
                .get(kind)
                .drained_by(ctx.config.need_decay_per_tick.get(kind));
            ctx.events.extend(pet.set_need(kind, next, ctx.tick));
        }
    }
});

system!(EmotionalUpdate, EmotionalUpdate, |ctx: &mut TickContext<
    '_,
>| {
    for pet in ctx.state.pets.values_mut().filter(|pet| pet.is_alive()) {
        pet.refresh_emotions();
    }
});

system!(HealthUpdate, HealthUpdate, |ctx: &mut TickContext<'_>| {
    let damage = Milli(ctx.config.neglect_damage_per_tick);
    for pet in ctx.state.pets.values_mut().filter(|pet| pet.is_alive()) {
        let (lowest_kind, lowest) = pet.needs.lowest();
        if lowest == NeedValue::EMPTY {
            if pet.health.current() <= damage {
                ctx.events
                    .extend(pet.die(DeathCause::Neglect { need: lowest_kind }, ctx.tick));
            } else {
                pet.health.damage(damage);
            }
        }
    }
});

system!(IllnessUpdate, IllnessUpdate, |ctx: &mut TickContext<'_>| {
    let is_day = ctx.tick.is_boundary_of(GameDuration::from_days(1));
    for pet in ctx.state.pets.values_mut().filter(|pet| pet.is_alive()) {
        let illness_count = ctx.content.illnesses().len();
        if is_day && illness_count != 0 {
            let mut rng = rng_for(
                ctx.seed,
                RngStream::IllnessOnset,
                ctx.tick,
                pet.id.rng_key(),
            );
            if rng.chance(
                ctx.config.illness_roll_numerator_per_day,
                ctx.config.illness_roll_denominator,
            ) {
                let index = rng.below(illness_count as u64) as usize;
                let illness = ctx
                    .content
                    .illnesses()
                    .nth(index)
                    .expect("index is bounded");
                pet.health
                    .contract(illness.id.clone(), Severity::Mild, ctx.tick);
            }
            let active: Vec<_> = pet
                .health
                .ailments()
                .iter()
                .map(|a| a.illness.clone())
                .collect();
            for illness in active {
                pet.health.worsen(&illness);
            }
        }
        if pet.health.is_ill() {
            let damage = Milli(ctx.config.illness_damage_per_tick);
            if pet.health.current() <= damage {
                let illness = pet.health.ailments()[0].illness.clone();
                ctx.events
                    .extend(pet.die(DeathCause::Illness { illness }, ctx.tick));
            } else {
                pet.health.damage(damage);
            }
        }
    }
});

system!(Aging, Aging, |ctx: &mut TickContext<'_>| {
    for pet in ctx.state.pets.values_mut().filter(|pet| pet.is_alive()) {
        let days = pet.progression.age(ctx.tick).whole_days();
        let target = if days >= ctx.config.stages.elder_days {
            LifeStage::Elder
        } else if days >= ctx.config.stages.adult_days {
            LifeStage::Adult
        } else if days >= ctx.config.stages.teen_days {
            LifeStage::Teen
        } else if days >= ctx.config.stages.child_days {
            LifeStage::Child
        } else {
            LifeStage::Hatchling
        };
        if target > pet.stage
            && let Ok(events) = pet.advance_stage(target, ctx.tick)
        {
            ctx.events.extend(events);
        }
    }
});

system!(Growth, Growth, |ctx: &mut TickContext<'_>| {
    for pet in ctx.state.pets.values_mut().filter(|pet| pet.is_alive()) {
        pet.progression
            .grant(Experience(ctx.config.passive_experience_per_tick));
    }
});

system!(Evolution, Evolution, |ctx: &mut TickContext<'_>| {
    for pet in ctx.state.pets.values_mut().filter(|pet| pet.is_alive()) {
        if let Some(edge) = ctx.content.evolutions().find(|edge| {
            edge.from == pet.species && pet.progression.level().get() >= edge.minimum_level
        }) {
            pet.species = edge.to.clone();
        }
    }
});

system!(Cooldowns, Cooldowns, |ctx: &mut TickContext<'_>| {
    for pet in ctx.state.pets.values_mut() {
        if pet.activity.is_finished(ctx.tick) {
            pet.activity.finish(ctx.tick);
        }
    }
});

system!(Habits, Habits, |ctx: &mut TickContext<'_>| {
    if ctx.tick.0.is_multiple_of(TICKS_PER_DAY) {
        for pet in ctx.state.pets.values_mut() {
            pet.habits.decay_all(Milli(ctx.config.habit_decay_per_day));
        }
    }
});

system!(MemoryDecay, MemoryDecay, |ctx: &mut TickContext<'_>| {
    if ctx.tick.0.is_multiple_of(TICKS_PER_DAY) {
        for pet in ctx.state.pets.values_mut() {
            pet.memories.decay(Milli(ctx.config.memory_decay_per_day));
        }
    }
});

system!(Behavior, Behavior, |ctx: &mut TickContext<'_>| {
    if ctx.tick.0.is_multiple_of(TICKS_PER_DAY) {
        for pet in ctx.state.pets.values_mut().filter(|pet| pet.is_alive()) {
            let (need, _) = pet.needs.lowest();
            let activity = match need {
                NeedKind::Hunger => Activity::Eating,
                NeedKind::Energy => Activity::Sleeping,
                NeedKind::Hygiene => Activity::Bathing,
                NeedKind::Bladder => Activity::Idle,
                NeedKind::Social => Activity::Socialising,
                NeedKind::Fun => Activity::Playing,
            };
            pet.habits.reinforce(activity, Milli(250));
        }
    }
});

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn order_ids_match_their_systems() {
        assert_eq!(NeedsDecay::ID, SystemId::NeedsDecay);
        assert_eq!(Behavior::ID, SystemId::Behavior);
    }
}
