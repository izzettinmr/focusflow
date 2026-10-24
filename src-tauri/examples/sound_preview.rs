//! Explicit, local speaker check. Never opens the user's study database.
fn main() {
    let path = std::env::temp_dir().join(format!("focusflow-sound-preview-{}", std::process::id()));
    let player = focusflow_lib::sound::Player::new(&path).unwrap();
    for cue in ["start", "complete"] {
        player.preview(cue, 60).unwrap();
        println!("Played {cue}");
    }
}
