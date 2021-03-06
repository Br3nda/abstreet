use crate::game::Transition;
use crate::managed::WrappedComposite;
use crate::sandbox::gameplay::freeform::freeform_controller;
use crate::sandbox::gameplay::{GameplayMode, GameplayState};
use crate::ui::UI;
use ezgui::{EventCtx, GfxCtx};

pub struct PlayScenario;

impl PlayScenario {
    pub fn new(
        name: &String,
        ctx: &mut EventCtx,
        ui: &UI,
    ) -> (WrappedComposite, Box<dyn GameplayState>) {
        (
            freeform_controller(ctx, ui, GameplayMode::PlayScenario(name.to_string()), name),
            Box::new(PlayScenario),
        )
    }
}

impl GameplayState for PlayScenario {
    fn event(&mut self, _: &mut EventCtx, _: &mut UI) -> Option<Transition> {
        None
    }

    fn draw(&self, _: &mut GfxCtx, _: &UI) {}
}
