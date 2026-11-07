use super::EditFlow;
use crate::Edits;

/// The latest Sun position a person asked for, if any.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Sun(Option<f32>);

impl Edits for Sun {
    fn merge(self, newer: Self) -> Self {
        Self(newer.0.or(self.0))
    }

    fn is_empty(&self) -> bool {
        self.0.is_none()
    }
}

/// A render owner whose presentation state is lost with its device.
struct Owner {
    sun: f32,
}

impl Owner {
    const DEFAULT_SUN: f32 = 0.25;

    fn new() -> Self {
        Self {
            sun: Self::DEFAULT_SUN,
        }
    }

    /// Render one frame with its edits and read back the state it drew.
    fn render(&mut self, edits: Sun) -> f32 {
        if let Some(sun) = edits.0 {
            self.sun = sun;
        }
        self.sun
    }
}

/// The game's side: the controls model is the authority, and the panel shows it.
struct Game {
    desired: f32,
}

impl Game {
    fn observe(&mut self, readback: f32) {
        self.desired = readback;
    }

    fn replay(&self) -> Sun {
        Sun(Some(self.desired))
    }
}

#[test]
fn a_frame_in_flight_during_an_edit_cannot_revert_it() {
    let mut flow = EditFlow::<Sun>::default();
    flow.request();
    let before = flow.submitted();
    // Before any edit, every reply is observed.
    assert!(flow.admits(before));
    // A frame is in flight when the edit is queued.
    assert!(flow.queue(Sun(Some(0.5))));
    assert!(
        !flow.admits(before),
        "the stale reply reads back pre-edit state"
    );
    // The next submission carries the edit.
    assert_eq!(flow.pending(), Sun(Some(0.5)));
    let carrying = flow.submitted();
    assert!(flow.admits(carrying));
    assert_eq!(
        flow.pending(),
        Sun::default(),
        "submitted edits are not resent"
    );
    // Continuous input or a display target alone does not gate replies.
    flow.request();
    let later = flow.submitted();
    assert!(flow.admits(later));
    // Edits made just before a submission ride it without asking for another frame.
    flow.request();
    flow.carry(Sun(Some(0.6)));
    let carrying = flow.submitted();
    assert!(flow.admits(carrying));
    assert!(!flow.wants_frame());
    // Empty edits queue nothing.
    assert!(!flow.queue(Sun::default()));
    assert!(!flow.wants_frame());
}

#[test]
fn an_edit_cancelled_by_gpu_replacement_is_applied_after_recovery() {
    let mut flow = EditFlow::<Sun>::default();
    let mut owner = Owner::new();
    let mut game = Game {
        desired: Owner::DEFAULT_SUN,
    };

    // Frame A is in flight when the person drags the Sun.
    flow.request();
    let frame_a = flow.submitted();
    game.desired = 0.75;
    assert!(flow.queue(Sun(Some(0.75))));

    // A's reply reads back the old Sun and is not observed.
    let readback = owner.render(Sun::default());
    assert_eq!(readback, Owner::DEFAULT_SUN);
    if flow.admits(frame_a) {
        game.observe(readback);
    }
    assert_eq!(
        game.desired, 0.75,
        "the stale reply did not snap the panel back"
    );

    // Frame B carries the edit, but the device is lost before it is drawn. The app drops
    // B's reply by device generation, and the replacement owner starts from defaults.
    flow.request();
    let carried = flow.pending();
    assert_eq!(carried, Sun(Some(0.75)));
    let _frame_b = flow.submitted();
    flow.request();
    let mut owner = Owner::new();

    // The person edits again while the replacement is being built.
    game.desired = 0.5;
    assert!(flow.queue(Sun(Some(0.5))));
    assert!(flow.wants_frame());

    // Recovery completes and replays the whole desired state.
    flow.recovered(game.replay());
    let carried = flow.pending();
    let frame_c = flow.submitted();
    assert_eq!(carried, Sun(Some(0.5)));
    let readback = owner.render(carried);
    assert!(
        flow.admits(frame_c),
        "the first frame after recovery is observed"
    );
    game.observe(readback);
    assert_eq!(owner.sun, 0.5, "the replacement draws the latest edit");
    assert_eq!(game.desired, 0.5);
    assert!(!flow.wants_frame());
}
