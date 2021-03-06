use crate::common::Overlays;
use crate::game::{msg, Transition};
use crate::managed::WrappedComposite;
use crate::sandbox::gameplay::faster_trips::small_faster_trips_panel;
use crate::sandbox::gameplay::{
    challenge_controller, manage_overlays, GameplayMode, GameplayState,
};
use crate::ui::UI;
use ezgui::{hotkey, layout, EventCtx, GfxCtx, Key, ModalMenu};
use geom::{Duration, Statistic, Time};
use map_model::{IntersectionID, Map};
use sim::{BorderSpawnOverTime, OriginDestination, Scenario, TripMode};

pub struct FixTrafficSignals {
    time: Time,
    once: bool,
    menu: ModalMenu,
}

impl FixTrafficSignals {
    pub fn new(
        ctx: &mut EventCtx,
        mode: GameplayMode,
    ) -> (WrappedComposite, Box<dyn GameplayState>) {
        (
            challenge_controller(ctx, mode, "Traffic Signals Challenge"),
            Box::new(FixTrafficSignals {
                time: Time::START_OF_DAY,
                once: true,
                menu: ModalMenu::new(
                    "",
                    vec![
                        (hotkey(Key::F), "find slowest traffic signals"),
                        (hotkey(Key::D), "hide finished trip distribution"),
                        (hotkey(Key::S), "final score"),
                    ],
                    ctx,
                )
                .set_standalone_layout(layout::ContainerOrientation::TopLeftButDownABit(150.0)),
            }),
        )
    }
}

impl GameplayState for FixTrafficSignals {
    fn event(&mut self, ctx: &mut EventCtx, ui: &mut UI) -> Option<Transition> {
        // Once is never...
        if self.once {
            ui.overlay = Overlays::finished_trips_histogram(ctx, ui);
            self.once = false;
        }

        self.menu.event(ctx);

        // Technically this shows stop signs too, but mostly the bottlenecks are signals.
        if manage_overlays(
            &mut self.menu,
            ctx,
            ui,
            "find slowest traffic signals",
            "hide slowest traffic signals",
            match ui.overlay {
                Overlays::IntersectionDelay(_, _) => true,
                _ => false,
            },
        ) {
            ui.overlay = Overlays::intersection_delay(ctx, ui);
        }
        if manage_overlays(
            &mut self.menu,
            ctx,
            ui,
            "show finished trip distribution",
            "hide finished trip distribution",
            match ui.overlay {
                Overlays::FinishedTripsHistogram(_, _) => true,
                _ => false,
            },
        ) {
            ui.overlay = Overlays::finished_trips_histogram(ctx, ui);
        }

        if self.time != ui.primary.sim.time() {
            self.time = ui.primary.sim.time();
            self.menu
                .set_info(ctx, small_faster_trips_panel(TripMode::Drive, ui));
        }

        if self.menu.action("final score") {
            return Some(Transition::Push(msg("Final score", final_score(ui))));
        }

        if ui.primary.sim.time() >= Time::END_OF_DAY {
            // TODO Stop the challenge somehow
            return Some(Transition::Push(msg("Final score", final_score(ui))));
        }

        None
    }

    fn draw(&self, g: &mut GfxCtx, _: &UI) {
        self.menu.draw(g);
    }
}

fn final_score(ui: &UI) -> Vec<String> {
    let time = ui.primary.sim.time();
    let now = ui
        .primary
        .sim
        .get_analytics()
        .finished_trips(time, TripMode::Drive);
    let baseline = ui.prebaked().finished_trips(time, TripMode::Drive);
    // TODO Annoying to repeat this everywhere; any refactor possible?
    if now.count() == 0 || baseline.count() == 0 {
        return vec!["No data yet, run the simulation for longer".to_string()];
    }
    let now_50p = now.select(Statistic::P50);
    let baseline_50p = baseline.select(Statistic::P50);
    let mut lines = Vec::new();

    if time < Time::END_OF_DAY {
        lines.push(format!(
            "You have to run the simulation until the end of the day to get final results; {} to \
             go",
            Time::END_OF_DAY - time
        ));
    }

    if now_50p < baseline_50p - Duration::seconds(30.0) {
        lines.push(format!(
            "COMPLETED! 50%ile trip times are now {}, which is {} faster than the baseline {}",
            now_50p,
            baseline_50p - now_50p,
            baseline_50p
        ));
    } else if now_50p < baseline_50p {
        lines.push(format!(
            "Almost there! 50%ile trip times are now {}, which is {} faster than the baseline {}. \
             Can you reduce the times by 30s?",
            now_50p,
            baseline_50p - now_50p,
            baseline_50p
        ));
    } else if now_50p.epsilon_eq(baseline_50p) {
        lines.push(format!(
            "... Did you change anything? 50% ile trip times are {}, same as the baseline",
            now_50p
        ));
    } else {
        lines.push(format!(
            "Err... how did you make things WORSE?! 50%ile trip times are {}, which is {} slower \
             than the baseline {}",
            now_50p,
            now_50p - baseline_50p,
            baseline_50p
        ));
    }
    lines
}

// TODO Hacks in here, because I'm not convinced programatically specifying this is right. I think
// the Scenario abstractions and UI need to change to make this convenient to express in JSON / the
// UI.

// Motivate a separate left turn phase for north/south, but not left/right
pub fn tutorial_scenario_lvl1(map: &Map) -> Scenario {
    // TODO In lieu of the deleted labels
    let north = IntersectionID(2);
    let south = IntersectionID(3);
    // Hush, east/west is more cognitive overhead for me. >_<
    let left = IntersectionID(1);
    let right = IntersectionID(0);

    let mut s = Scenario::empty(map, "tutorial lvl1");

    // What's the essence of what I've specified below? Don't care about the time distribution,
    // exact number of agents, different modes. It's just an OD matrix with relative weights.
    //
    //        north  south  left  right
    // north   0      3      1     2
    // south   3      ... and so on
    // left
    // right
    //
    // The table isn't super easy to grok. But it motivates the UI for entering this info:
    //
    // 1) Select all of the sources
    // 2) Select all of the sinks (option to use the same set)
    // 3) For each (src, sink) pair, ask (none, light, medium, heavy)

    // Arterial straight
    heavy(&mut s, map, south, north);
    heavy(&mut s, map, north, south);
    // Arterial left turns
    medium(&mut s, map, south, left);
    medium(&mut s, map, north, right);
    // Arterial right turns
    light(&mut s, map, south, right);
    light(&mut s, map, north, left);

    // Secondary straight
    medium(&mut s, map, left, right);
    medium(&mut s, map, right, left);
    // Secondary right turns
    medium(&mut s, map, left, south);
    medium(&mut s, map, right, north);
    // Secondary left turns
    light(&mut s, map, left, north);
    light(&mut s, map, right, south);

    s
}

// Motivate a pedestrian scramble cycle
pub fn tutorial_scenario_lvl2(map: &Map) -> Scenario {
    let north = IntersectionID(3);
    let south = IntersectionID(3);
    let left = IntersectionID(1);
    let right = IntersectionID(0);

    let mut s = tutorial_scenario_lvl1(map);
    s.scenario_name = "tutorial lvl2".to_string();

    // TODO The first few phases aren't affected, because the peds walk slowly from the border.
    // Start them from a building instead?
    // TODO All the peds get through in a single wave; spawn them continuously?
    // TODO The metrics shown are just for driving trips...
    heavy_peds(&mut s, map, south, north);
    heavy_peds(&mut s, map, north, south);
    heavy_peds(&mut s, map, left, right);
    heavy_peds(&mut s, map, right, left);

    s
}

fn heavy(s: &mut Scenario, map: &Map, from: IntersectionID, to: IntersectionID) {
    spawn(s, map, from, to, 100, 0);
}
fn heavy_peds(s: &mut Scenario, map: &Map, from: IntersectionID, to: IntersectionID) {
    spawn(s, map, from, to, 0, 100);
}
fn medium(s: &mut Scenario, map: &Map, from: IntersectionID, to: IntersectionID) {
    spawn(s, map, from, to, 100, 0);
}
fn light(s: &mut Scenario, map: &Map, from: IntersectionID, to: IntersectionID) {
    spawn(s, map, from, to, 100, 0);
}

fn spawn(
    s: &mut Scenario,
    map: &Map,
    from: IntersectionID,
    to: IntersectionID,
    num_cars: usize,
    num_peds: usize,
) {
    s.border_spawn_over_time.push(BorderSpawnOverTime {
        num_peds,
        num_cars,
        num_bikes: 0,
        percent_use_transit: 0.0,
        start_time: Time::START_OF_DAY,
        stop_time: Time::START_OF_DAY + Duration::minutes(5),
        start_from_border: map.get_i(from).some_outgoing_road(map),
        goal: OriginDestination::EndOfRoad(map.get_i(to).some_incoming_road(map)),
    });
}
