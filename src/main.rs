use bevy::prelude::*;
use itertools::Itertools;
use rand::prelude::*;
use std::cmp::Ordering;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "2048".to_string(),
                ..default()
            }),
            ..default()
        }))
        .add_event::<NewTileEvent>()
        .add_systems(Startup, (setup, spawn_playground, spawn_tiles).chain())
        .add_systems(
            Update,
            (
                render_tile_points,
                move_tiles,
                render_tiles,
                new_tile_handler,
            ),
        )
        .run()
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

const TILE_SIZE: f32 = 100.0;
const TILE_SPACER: f32 = 10.0;

#[derive(Component)]
struct Points {
    value: u32,
}

#[derive(Component, PartialEq, Clone, Copy)]
struct Position {
    x: u8,
    y: u8,
}

#[derive(Component)]
struct Playground {
    grid: u8,
    size: f32,
}

#[derive(Component)]
struct TileText;

impl Playground {
    fn new(grid: u8) -> Self {
        let size = f32::from(grid) * TILE_SIZE + f32::from(grid + 1) * TILE_SPACER;
        Playground { grid, size }
    }
    fn tile_pos(&self, pos: u8) -> f32 {
        let offset = -self.size / 2.0 + 0.5 * TILE_SIZE;
        offset + f32::from(pos) * TILE_SIZE + f32::from(pos + 1) * TILE_SPACER
    }
}

fn spawn_playground(mut commands: Commands) {
    let playground = Playground::new(4);
    commands
        .spawn(SpriteBundle {
            sprite: Sprite {
                color: Color::hex("#e4b4b7").unwrap(),
                custom_size: Some(Vec2::new(playground.size, playground.size)),
                ..default()
            },
            ..default()
        })
        .with_children(|builder| {
            for tile in (0..playground.grid).cartesian_product(0..playground.grid) {
                builder.spawn(SpriteBundle {
                    sprite: Sprite {
                        custom_size: Some(Vec2::new(TILE_SIZE, TILE_SIZE)),
                        ..default()
                    },
                    transform: Transform::from_xyz(
                        playground.tile_pos(tile.0),
                        playground.tile_pos(tile.1),
                        0.1,
                    ),
                    ..default()
                });
            }
        })
        .insert(playground);
}

fn spawn_tiles(mut commands: Commands, query_playground: Query<&Playground>) {
    let playground = query_playground.single();
    let mut rng = rand::thread_rng();
    let starting_tiles: Vec<(u8, u8)> = (0..playground.grid)
        .cartesian_product(0..playground.grid)
        .choose_multiple(&mut rng, 2);

    for (x, y) in starting_tiles.iter() {
        let pos = Position { x: *x, y: *y };
        spawn_tile(&mut commands, playground, pos)
    }
}

fn render_tile_points(
    mut texts: Query<&mut Text, With<TileText>>,
    tiles: Query<(&Points, &Children)>,
) {
    for (points, children) in tiles.iter() {
        if let Some(entity) = children.first() {
            let mut text = texts.get_mut(*entity).expect("expected text");
            let text_section = text.sections.first_mut().expect("expected editable");
            text_section.value = points.value.to_string()
        }
    }
}

enum MoveTiles {
    Left,
    Right,
    Up,
    Down,
}

impl MoveTiles {
    fn sort(&self, a: &Position, b: &Position) -> Ordering {
        match self {
            MoveTiles::Left => match Ord::cmp(&a.y, &b.y) {
                Ordering::Equal => Ord::cmp(&a.x, &b.x),
                ordering => ordering,
            },
            MoveTiles::Right => match Ord::cmp(&b.y, &a.y) {
                Ordering::Equal => Ord::cmp(&b.x, &a.x),
                ordering => ordering,
            },
            MoveTiles::Up => match Ord::cmp(&b.x, &a.x) {
                Ordering::Equal => Ord::cmp(&b.y, &a.y),
                ordering => ordering,
            },
            MoveTiles::Down => match Ord::cmp(&a.x, &b.x) {
                Ordering::Equal => Ord::cmp(&a.y, &b.y),
                ordering => ordering,
            },
        }
    }
    fn set_column(&self, playground_grid: u8, position: &mut Mut<Position>, index: u8) {
        match self {
            MoveTiles::Left => {
                position.x = index;
            }
            MoveTiles::Right => {
                position.x = playground_grid - 1 - index;
            }
            MoveTiles::Up => {
                position.y = playground_grid - 1 - index;
            }
            MoveTiles::Down => {
                position.y = index;
            }
        }
    }
    fn get_row(&self, position: &Position) -> u8 {
        match self {
            MoveTiles::Left | MoveTiles::Right => position.y,
            MoveTiles::Up | MoveTiles::Down => position.x,
        }
    }
}

impl TryFrom<&KeyCode> for MoveTiles {
    type Error = &'static str;

    fn try_from(value: &KeyCode) -> Result<Self, Self::Error> {
        match value {
            KeyCode::ArrowLeft => Ok(MoveTiles::Left),
            KeyCode::ArrowRight => Ok(MoveTiles::Right),
            KeyCode::ArrowUp => Ok(MoveTiles::Up),
            KeyCode::ArrowDown => Ok(MoveTiles::Down),
            _ => Err("please use arrow keys"),
        }
    }
}

fn move_tiles(
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    mut tiles: Query<(Entity, &mut Position, &mut Points)>,
    query_playground: Query<&Playground>,
    mut tile_writer: EventWriter<NewTileEvent>,
) {
    let playground = query_playground.single();
    let shift_direction = input
        .get_just_pressed()
        .find_map(|key_code| MoveTiles::try_from(key_code).ok());
    if let Some(move_tiles) = shift_direction {
        let mut it = tiles
            .iter_mut()
            .sorted_by(|a, b| move_tiles.sort(&a.1, &b.1))
            .peekable();
        let mut column: u8 = 0;
        while let Some(mut tile) = it.next() {
            move_tiles.set_column(playground.grid, &mut tile.1, column);
            if let Some(peeked_tile) = it.peek() {
                if move_tiles.get_row(&tile.1) != move_tiles.get_row(&peeked_tile.1) {
                    column = 0;
                } else if tile.2.value != peeked_tile.2.value {
                    column += 1;
                } else {
                    let next_tile = it.next().expect("expected peeked tile");
                    tile.2.value *= 2;
                    commands.entity(next_tile.0).despawn_recursive();
                    if let Some(more_tile) = it.peek() {
                        if move_tiles.get_row(&tile.1) != move_tiles.get_row(&more_tile.1) {
                            column = 0;
                        } else {
                            column += 1;
                        }
                    }
                }
            }
        }
        tile_writer.send(NewTileEvent);
    }
}

fn render_tiles(
    mut tiles: Query<(&mut Transform, &Position), Changed<Position>>,
    query_playground: Query<&Playground>,
) {
    let playground = query_playground.single();
    for (mut transform, pos) in tiles.iter_mut() {
        transform.translation.x = playground.tile_pos(pos.x);
        transform.translation.y = playground.tile_pos(pos.y);
    }
}

#[derive(Event)]
struct NewTileEvent;

fn new_tile_handler(
    mut tile_reader: EventReader<NewTileEvent>,
    mut commands: Commands,
    query_playground: Query<&Playground>,
    tiles: Query<&Position>,
) {
    let playground = query_playground.single();

    for _ in tile_reader.read() {
        let mut rng = rand::thread_rng();
        let possible_position: Option<Position> = (0..playground.grid)
            .cartesian_product(0..playground.grid)
            .filter_map(|tile_pos| {
                let new_pos = Position {
                    x: tile_pos.0,
                    y: tile_pos.1,
                };
                match tiles.iter().find(|&&pos| pos == new_pos) {
                    Some(_) => None,
                    None => Some(new_pos),
                }
            })
            .choose(&mut rng);

        if let Some(pos) = possible_position {
            spawn_tile(&mut commands, playground, pos);
        }
    }
}

fn spawn_tile(commands: &mut Commands, playground: &Playground, pos: Position) {
    commands
        .spawn(SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::new(TILE_SIZE, TILE_SIZE)),
                ..default()
            },
            transform: Transform::from_xyz(
                playground.tile_pos(pos.x),
                playground.tile_pos(pos.y),
                2.0,
            ),
            ..default()
        })
        .with_children(|child_builder| {
            child_builder
                .spawn(Text2dBundle {
                    text: Text::from_section(
                        "2",
                        TextStyle {
                            font_size: 50.0,
                            color: Color::hex("#c35048").unwrap(),
                            ..default()
                        },
                    ),
                    transform: Transform::from_xyz(0.0, 0.0, 1.0),
                    ..default()
                })
                .insert(TileText);
        })
        .insert(Points { value: 2 })
        .insert(pos);
}
