use minesweeper_rs::*;

use bevy::{log::LogPlugin, prelude::*};

#[cfg(feature = "debug")]
use bevy_inspector_egui::WorldInspectorPlugin;

fn main() {
    // tracing_subscriber::fmt().init();

    tracing::debug!("This might appear in logs");

    App::new()
        .add_plugins((DefaultPlugins.set(LogPlugin {
            level: bevy::log::Level::DEBUG,
            filter: "info,minesweeper_rs=debug".into(),
            ..default()
        }),))
        .add_systems(Startup, setup_grid)
        .add_systems(
            OnEnter(GameState::Playing),
            (
                fill_with_mines,
                // attach_labels_to_mines_near,
                flood_reveal_initial,
            )
                .chain(),
        )
        .add_systems(OnEnter(GameState::GameOver), reveal_all_mines)
        .init_state::<GameState>()
        .run();
}
