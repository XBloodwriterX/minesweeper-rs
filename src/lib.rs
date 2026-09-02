use std::{
    collections::{BTreeSet, HashMap, HashSet},
    sync::Arc,
};

use bevy::{
    ecs::{query::QueryData, system::RunSystemOnce},
    picking::prelude::*,
    prelude::*,
    sprite,
};

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameState {
    #[default]
    Setup, // placing mines, preparing board
    Playing,  // normal gameplay
    GameOver, // mine hit
    Won,      // all safe tiles revealed
}

#[derive(Component, Default, Debug)]
pub struct Tile {
    pub col: usize,
    pub row: usize,
    pub label: Option<Entity>,
}

#[derive(Component)]
pub struct Mine;

#[derive(Component)]
pub struct MinesNear(u8);

#[derive(Component)]
pub struct Revealed;

#[derive(Component)]
pub struct Flagged;

// #[derive(Resource)]
// pub struct FirstClick {
//     pub entity: Entity,
//     pub col: usize,
//     pub row: usize,
// }

#[derive(QueryData)]
#[query_data(mutable)]
pub struct TileData {
    pub entity: Entity,
    pub tile: &'static Tile,
    pub sprite: &'static mut Sprite,
    pub has_mine: Has<Mine>,
    pub has_mines_near: Has<MinesNear>,
}

// type TileQuery<'w, 's> = Query<
//     'static,
//     'static,
//     (
//         Entity,
//         &'static Tile,
//         &'static mut Sprite,
//         Has<Mine>,
//         Has<MinesNear>,
//     ),
//     Without<Revealed>,
// >;

const TILE_SIZE: f32 = 50.0;
const TILE_GAP: f32 = 2.0;
const COLS: usize = 10;
const ROWS: usize = 10;
const MINES: usize = 10;
// const GRID_WIDTH: usize = 10;
// const GRID_HEIGHT: usize = 10;

pub fn setup_grid(mut commands: Commands) {
    commands.spawn((Camera2d, SpritePickingCamera));

    let step = TILE_SIZE + TILE_GAP;
    let offset_x = (COLS as f32 - 1.0) * step / 2.0;
    let offset_y = (ROWS as f32 - 1.0) * step / 2.0;

    for row in 0..ROWS {
        for col in 0..COLS {
            let x = col as f32 * step - offset_x;
            let y = row as f32 * step - offset_y;

            commands
                .spawn((
                    Tile {
                        col,
                        row,
                        ..default()
                    },
                    Sprite {
                        color: Color::srgb(0.4, 0.6, 0.9),
                        custom_size: Some(Vec2::splat(TILE_SIZE)),
                        ..default()
                    },
                    Transform::from_xyz(x, y, 0.0),
                    Pickable::default(),
                ))
                .observe(on_tile_hover)
                .observe(on_tile_out)
                .observe(on_tile_clicked);
        }
    }
}

fn on_tile_hover(
    event: On<Pointer<Over>>,
    mut sprite_query: Query<&mut Sprite, (With<Tile>, Without<Revealed>)>,
) {
    if let Ok(mut sprite) = sprite_query.get_mut(event.entity) {
        sprite.color = Color::srgb(0.2, 0.3, 0.45);
    }
}

fn on_tile_out(
    event: On<Pointer<Out>>,
    mut sprite_query: Query<&mut Sprite, (With<Tile>, Without<Revealed>)>,
) {
    if let Ok(mut sprite) = sprite_query.get_mut(event.entity) {
        sprite.color = Color::srgb(0.4, 0.6, 0.9);
    }
}

fn on_tile_clicked(
    event: On<Pointer<Press>>,
    state: Res<State<GameState>>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    mut tile_query: Query<TileData, Without<Revealed>>,
    mut text_query: Query<&mut Visibility, (With<Text2d>, With<ChildOf>)>,
) {
    if event.button != PointerButton::Primary {
        return;
    }

    let entity = event.entity;

    let Ok(mut t) = tile_query.get_mut(entity) else {
        return;
    };

    let col = t.tile.col;
    let row = t.tile.row;

    match *state.get() {
        GameState::Setup => {
            reveal_tile(
                &mut commands,
                entity,
                t.tile,
                &mut t.sprite,
                &mut text_query,
            );
            drop(t.sprite);
            next_state.set(GameState::Playing);
            // flood_reveal(
            //     col,
            //     row,
            //     &mut commands,
            //     &mut tile_query,
            //     &mut text_query,
            //     &mut visited,
            // );
        }
        GameState::Playing => {
            if t.has_mine {
                next_state.set(GameState::GameOver);
            } else {
                reveal_tile(
                    &mut commands,
                    entity,
                    t.tile,
                    &mut t.sprite,
                    &mut text_query,
                );
                drop(t.sprite);
                if !t.has_mines_near {
                    let mut visited = HashSet::new();
                    visited.insert(entity);
                    flood_reveal(
                        col,
                        row,
                        &mut commands,
                        &mut tile_query,
                        &mut text_query,
                        &mut visited,
                    );
                }
            }
        }
        _ => {}
    }
}

pub fn flood_reveal_initial(
    first_click: Single<(Entity, &Tile), With<Revealed>>,
    mut commands: Commands,
    mut tile_query: Query<TileData, Without<Revealed>>,
    mut text_query: Query<&mut Visibility, (With<Text2d>, With<ChildOf>)>,
) {
    let (entity, tile) = *first_click;
    let mut visited = HashSet::new();
    visited.insert(entity);
    flood_reveal(
        tile.col,
        tile.row,
        &mut commands,
        &mut tile_query,
        &mut text_query,
        &mut visited,
    );
}

fn flood_reveal(
    x: usize,
    y: usize,
    commands: &mut Commands,
    tile_query: &mut Query<TileData, Without<Revealed>>,
    text_query: &mut Query<&mut Visibility, (With<Text2d>, With<ChildOf>)>,
    visited: &mut HashSet<Entity>,
) {
    let neighbors: Vec<_> = tile_query
        .iter()
        .filter(|t| {
            let col_dist = x.abs_diff(t.tile.col);
            let row_dist = y.abs_diff(t.tile.row);
            col_dist <= 1
                && row_dist <= 1
                && !(col_dist == 0 && row_dist == 0)
                && !t.has_mine
                && !visited.contains(&t.entity)
        })
        .map(|t| (t.entity, t.tile.col, t.tile.row, t.has_mines_near))
        .collect();

    let entities: HashSet<Entity> = neighbors
        .iter()
        .map(|(e, _, _, _)| {
            visited.insert(*e);
            *e
        })
        .collect();

    {
        let mut iter = tile_query.iter_many_mut(&entities);
        while let Some(mut t) = iter.fetch_next() {
            reveal_tile(commands, t.entity, t.tile, &mut t.sprite, text_query);
        }
    }

    for (_, col, row, is_mines_near) in &neighbors {
        if !is_mines_near {
            flood_reveal(*col, *row, commands, tile_query, text_query, visited);
        }
    }
}

pub fn fill_with_mines(
    mut commands: Commands,
    mut tile_query: Query<(Entity, &mut Tile), (Without<Revealed>, Without<Mine>)>,
    first_tile: Single<&Tile, With<Revealed>>,
) {
    use rand::{rng, seq::IteratorRandom};

    let mut rng = rng();

    let mines_entities: HashSet<Entity> = tile_query
        .iter()
        .filter(|(_, tile)| {
            let dc = tile.col.abs_diff(first_tile.col);
            let dr = tile.row.abs_diff(first_tile.row);
            dc > 1 || dr > 1
        })
        .map(|(entity, _)| entity)
        .sample(&mut rng, MINES)
        .into_iter()
        .collect();

    let candidates: HashMap<(usize, usize), Entity> = tile_query
        .iter()
        .map(|(entity, tile)| ((tile.col, tile.row), entity))
        .collect();
    // let candidates: HashMap<(usize, usize), (Entity, &mut Tile)> = tile_query
    //     .iter_mut()
    //     .map(|(entity, mut tile)| ((tile.col, tile.row), (entity, tile.as_mut())))
    //     .collect();

    let mut mines_near: HashMap<Entity, u8> = HashMap::new();

    {
        let mut iter = tile_query.iter_many_mut(&mines_entities);
        while let Some((mine_entity, mut mine_tile)) = iter.fetch_next() {
            let label = commands
                .spawn((
                    // Text2d::new("!"),
                    Text2d::new("🚩"),
                    Transform::from_xyz(0.0, 0.0, 1.0),
                    Visibility::Hidden,
                ))
                .id();
            mine_tile.label = Some(label);
            commands.entity(mine_entity).insert(Mine).add_child(label);

            let mine_col = mine_tile.col;
            let mine_row = mine_tile.row;
            for dc in (mine_col.max(1) - 1)..=(mine_col + 1).min(COLS - 1) {
                for dr in (mine_row.max(1) - 1)..=(mine_row + 1).min(ROWS - 1) {
                    if dc == dr {
                        continue;
                    }
                    if let Some(&neighbor) = candidates.get(&(dc, dr)) {
                        if !mines_entities.contains(&neighbor) {
                            *mines_near.entry(neighbor).or_insert(0) += 1;
                        }
                    }
                }
            }
        }
    }

    {
        let mut iter = tile_query.iter_many_mut(mines_near.keys());
        while let Some((mn_entity, mut mn_tile)) = iter.fetch_next() {
            let count = mines_near[&mn_entity];
            let label = commands
                .spawn((
                    Text2d::new(count.to_string()),
                    TextColor(Color::hsl(40.0 * count as f32, 0.9, 0.9)),
                    Transform::from_xyz(0.0, 0.0, 1.0),
                    Visibility::Hidden,
                ))
                .id();
            mn_tile.label = Some(label);
            commands
                .entity(mn_entity)
                .add_child(label)
                .entry::<MinesNear>()
                .or_insert(MinesNear(count));
        }
    }
}

pub fn attach_labels_to_mines_near(
    mut commands: Commands,
    mines_near_query: Query<(Entity, &mut Tile, &MinesNear)>,
) {
    for (entity, mut tile, mines_near) in mines_near_query {
        let label = commands
            .spawn((
                Text2d::new(mines_near.0.to_string()),
                TextColor(Color::hsl(40.0 * mines_near.0 as f32, 0.9, 0.9)),
                Transform::from_xyz(0.0, 0.0, 1.0),
                Visibility::Hidden,
            ))
            .id();
        tile.label = Some(label);
        commands.entity(entity).add_child(label);
    }
}

// fn reveal_label(
//     entity: Entity,
//     text_query: &mut Query<&mut Visibility, (With<Text2d>, With<ChildOf>)>,
// ) {
// if let Ok(mut t) = text_query.get_mut(entity) {
//     *t = Visibility::Visible;
// }
// }

fn reveal_tile(
    commands: &mut Commands,
    entity: Entity,
    tile: &Tile,
    sprite: &mut Sprite,
    text_query: &mut Query<&mut Visibility, (With<Text2d>, With<ChildOf>)>,
) {
    sprite.color = Color::srgb(0.85, 0.85, 0.85);
    commands.entity(entity).insert(Revealed);
    if let Some(label_entity) = tile.label {
        // reveal_label(label_entity, text_query);
        if let Ok(mut t) = text_query.get_mut(label_entity) {
            *t = Visibility::Visible;
        }
    }
}

#[tracing::instrument(skip(commands))]
fn reveal_mine(
    commands: &mut Commands,
    entity: Entity,
    tile: &Tile,
    sprite: &mut Sprite,
    text_query: &mut Query<&mut Visibility, (With<Text2d>, With<ChildOf>)>,
) {
    sprite.color = Color::srgb(0.9, 0.2, 0.2);
    commands.entity(entity).insert(Revealed);
    match tile.label {
        Some(label_entity) => {
            // reveal_label(label_entity, text_query);
            if let Ok(mut t) = text_query.get_mut(label_entity) {
                *t = Visibility::Visible;
            }
        }
        None => tracing::error!("Mine({:?}) hasn't a label", tile),
    }
}

pub fn reveal_all_mines(
    mut commands: Commands,
    mine_query: Query<(Entity, &mut Sprite, &Tile), (With<Mine>, With<Children>)>,
    mut text_query: Query<&mut Visibility, (With<Text2d>, With<ChildOf>)>,
) {
    for (entity, mut sprite, tile) in mine_query {
        reveal_mine(&mut commands, entity, tile, &mut sprite, &mut text_query);
    }
}

// pub fn controls(input: Res<ButtonInput<MouseButton>>) {}
