use crate::common::Overlays;
use crate::game::{Transition, WizardState};
use crate::managed::WrappedComposite;
use crate::sandbox::gameplay::{
    challenge_controller, cmp_duration_shorter, manage_overlays, GameplayMode, GameplayState,
};
use crate::sandbox::SandboxMode;
use crate::ui::UI;
use ezgui::{hotkey, layout, Choice, EventCtx, GfxCtx, Key, Line, ModalMenu, Text};
use geom::{Statistic, Time};
use map_model::BusRouteID;

pub struct OptimizeBus {
    route: BusRouteID,
    time: Time,
    stat: Statistic,
    menu: ModalMenu,
}

impl OptimizeBus {
    pub fn new(
        route_name: String,
        ctx: &mut EventCtx,
        ui: &UI,
    ) -> (WrappedComposite, Box<dyn GameplayState>) {
        let route = ui.primary.map.get_bus_route(&route_name).unwrap();
        (
            challenge_controller(
                ctx,
                GameplayMode::OptimizeBus(route_name.clone()),
                &format!("Optimize {} Challenge", route_name),
            ),
            Box::new(OptimizeBus {
                route: route.id,
                time: Time::START_OF_DAY,
                stat: Statistic::Max,
                menu: ModalMenu::new(
                    "",
                    vec![
                        (hotkey(Key::E), "show bus route"),
                        (hotkey(Key::T), "show delays over time"),
                        (hotkey(Key::P), "show bus passengers"),
                        (hotkey(Key::S), "change statistic"),
                    ],
                    ctx,
                )
                .set_standalone_layout(layout::ContainerOrientation::TopLeftButDownABit(150.0)),
            }),
        )
    }
}

impl GameplayState for OptimizeBus {
    fn event(&mut self, ctx: &mut EventCtx, ui: &mut UI) -> Option<Transition> {
        self.menu.event(ctx);
        if manage_overlays(
            &mut self.menu,
            ctx,
            ui,
            "show bus route",
            "hide bus route",
            match ui.overlay {
                Overlays::BusRoute(_, ref r, _) => *r == self.route,
                _ => false,
            },
        ) {
            ui.overlay = Overlays::show_bus_route(self.route, ctx, ui);
        }
        if manage_overlays(
            &mut self.menu,
            ctx,
            ui,
            "show delays over time",
            "hide delays over time",
            match ui.overlay {
                Overlays::BusDelaysOverTime(_, ref r, _) => *r == self.route,
                _ => false,
            },
        ) {
            ui.overlay = Overlays::delays_over_time(self.route, ctx, ui);
        }
        if manage_overlays(
            &mut self.menu,
            ctx,
            ui,
            "show bus passengers",
            "hide bus passengers",
            match ui.overlay {
                Overlays::BusPassengers(_, ref r, _) => *r == self.route,
                _ => false,
            },
        ) {
            ui.overlay = Overlays::bus_passengers(self.route, ctx, ui);
        }

        // TODO Expensive
        if self.time != ui.primary.sim.time() {
            self.time = ui.primary.sim.time();
            self.menu
                .set_info(ctx, bus_route_panel(self.route, self.stat, ui));
        }

        if self.menu.action("change statistic") {
            return Some(Transition::Push(WizardState::new(Box::new(
                move |wiz, ctx, _| {
                    // TODO Filter out existing. Make this kind of thing much easier.
                    let (_, new_stat) = wiz.wrap(ctx).choose(
                        "Show which statistic on frequency a bus stop is visited?",
                        || {
                            Statistic::all()
                                .into_iter()
                                .map(|s| Choice::new(s.to_string(), s))
                                .collect()
                        },
                    )?;
                    Some(Transition::PopWithData(Box::new(move |state, _, _| {
                        let sandbox = state.downcast_mut::<SandboxMode>().unwrap();
                        let opt = sandbox
                            .gameplay
                            .state
                            .downcast_mut::<OptimizeBus>()
                            .unwrap();
                        // Force recalculation
                        opt.time = Time::START_OF_DAY;
                        opt.stat = new_stat;
                    })))
                },
            ))));
        }
        None
    }

    fn draw(&self, g: &mut GfxCtx, _: &UI) {
        self.menu.draw(g);
    }
}

fn bus_route_panel(id: BusRouteID, stat: Statistic, ui: &UI) -> Text {
    let now = ui
        .primary
        .sim
        .get_analytics()
        .bus_arrivals(ui.primary.sim.time(), id);
    let baseline = ui.prebaked().bus_arrivals(ui.primary.sim.time(), id);

    let route = ui.primary.map.get_br(id);
    let mut txt = Text::new();
    txt.add(Line(format!("{} delay between stops", stat)));
    for idx1 in 0..route.stops.len() {
        let idx2 = if idx1 == route.stops.len() - 1 {
            0
        } else {
            idx1 + 1
        };
        // TODO Also display number of arrivals...
        txt.add(Line(format!("Stop {}->{}: ", idx1 + 1, idx2 + 1)));
        if let Some(ref stats1) = now.get(&route.stops[idx2]) {
            let a = stats1.select(stat);
            txt.append(Line(a.to_string()));

            if let Some(ref stats2) = baseline.get(&route.stops[idx2]) {
                txt.append_all(cmp_duration_shorter(a, stats2.select(stat)));
            }
        } else {
            txt.append(Line("no arrivals yet"));
        }
    }
    txt
}
