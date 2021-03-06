use std::time::Duration;

use bevy::{prelude::*, render::pass::ClearColor};
use rand::prelude::random;

const ARENA_WIDTH: u32 = 40;
const ARENA_HEIGHT: u32 = 40;

#[derive(Default, Copy, Clone, Debug, Eq, PartialEq, Hash)]
struct Position {
    x: i32,
    y: i32,
}

struct Size {
    width: f32,
    height: f32,
}

impl Size {
    pub fn square(x: f32) -> Self {
        Self {
            width: x,
            height: x,
        }
    }
}

#[derive(PartialEq, Copy, Clone, Debug)]
enum Direction {
    Left,
    Up,
    Right,
    Down,
}

impl Direction {
    fn opposite(self: &Self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::Up => Self::Down,
            Self::Down => Self::Up,
        }
    }
}

struct SnakeHead {
    direction: Direction,
    next_segment: Entity,
}

struct SnakeSegment {
    next_segment: Option<Entity>,
}

struct Food;

struct HeadMaterial(Handle<ColorMaterial>);
struct SegmentMaterial(Handle<ColorMaterial>);
struct FoodMaterial(Handle<ColorMaterial>);

struct SnakeMoveTimer(Timer);
struct FoodSpawnTimer(Timer);

struct GameOverEvent;

fn spawn_segment(
    commands: &mut Commands,
    material: Handle<ColorMaterial>,
    position: Position,
) -> Entity {
    commands
        .spawn(SpriteComponents {
            material,
            ..Default::default()
        })
        .with(SnakeSegment { next_segment: None })
        .with(position)
        .with(Size::square(0.65));
    commands.current_entity().unwrap()
}

fn spawn_initial_snake(
    mut commands: &mut Commands,
    head_material: Res<HeadMaterial>,
    segment_material: Res<SegmentMaterial>,
) {
    let first_segment = spawn_segment(&mut commands, segment_material.0, Position { x: 10, y: 9 });
    commands
        .spawn(SpriteComponents {
            material: head_material.0,
            ..Default::default()
        })
        .with(SnakeHead {
            direction: Direction::Up,
            next_segment: first_segment,
        })
        .with(Position { x: 10, y: 10 })
        .with(Size::square(0.8));
}

fn setup(mut commands: Commands, mut materials: ResMut<Assets<ColorMaterial>>) {
    commands.spawn(Camera2dComponents::default());
    commands.insert_resource(HeadMaterial(
        materials.add(Color::rgb(0.7, 0.7, 0.7).into()),
    ));
    commands.insert_resource(SegmentMaterial(
        materials.add(Color::rgb(0.3, 0.3, 0.3).into()),
    ));
    commands.insert_resource(FoodMaterial(
        materials.add(Color::rgb(1.0, 0.0, 1.0).into()),
    ));
}

fn game_setup(
    mut commands: Commands,
    head_material: Res<HeadMaterial>,
    segment_material: Res<SegmentMaterial>,
) {
    spawn_initial_snake(&mut commands, head_material, segment_material);
}

fn snake_movement(
    mut commands: Commands,
    time: Res<Time>,
    keyboard_input: Res<Input<KeyCode>>,
    mut snake_timer: ResMut<SnakeMoveTimer>,
    segment_material: Res<SegmentMaterial>,
    mut game_over_events: ResMut<Events<GameOverEvent>>,
    mut head_positions: Query<(&mut SnakeHead, &mut Position)>,
    segments: Query<&mut SnakeSegment>,
    positions: Query<&mut Position>,
    mut food_positions: Query<(Entity, &Food, &Position)>,
) {
    snake_timer.0.tick(time.delta_seconds);
    for (mut head, mut head_pos) in &mut head_positions.iter() {
        let mut dir = head.direction;
        if keyboard_input.pressed(KeyCode::Left) {
            dir = Direction::Left;
        }
        if keyboard_input.pressed(KeyCode::Right) {
            dir = Direction::Right;
        }
        if keyboard_input.pressed(KeyCode::Down) {
            dir = Direction::Down;
        }
        if keyboard_input.pressed(KeyCode::Up) {
            dir = Direction::Up;
        }

        if dir != head.direction.opposite() {
            head.direction = dir;
        }

        if snake_timer.0.finished {
            // Move each tail segment to its parent's position.
            // TODO: Try out the algorithm where we instead just move the last segment to the
            // head's current position and update the pointer to the last segment.
            let mut last_position = *head_pos;
            let mut segment_entity = head.next_segment;
            loop {
                let segment = segments.get::<SnakeSegment>(segment_entity).unwrap();
                let mut segment_position = positions.get_mut::<Position>(segment_entity).unwrap();
                let current_position = *segment_position;
                *segment_position = last_position;
                last_position = current_position;

                // Check if we hit our own tail.
                if *head_pos == last_position {
                    game_over_events.send(GameOverEvent);
                }

                if let Some(next) = segment.next_segment {
                    segment_entity = next;
                } else {
                    break;
                }
            }

            // Move the head segment.
            match head.direction {
                Direction::Left => {
                    head_pos.x -= 1;
                }
                Direction::Right => {
                    head_pos.x += 1;
                }
                Direction::Up => {
                    head_pos.y += 1;
                }
                Direction::Down => {
                    head_pos.y -= 1;
                }
            }

            // Check if we hit a wall.
            if head_pos.x < 0
                || head_pos.y < 0
                || head_pos.x as u32 > ARENA_WIDTH
                || head_pos.y as u32 > ARENA_HEIGHT
            {
                game_over_events.send(GameOverEvent);
            }

            for (ent, _food, food_pos) in &mut food_positions.iter() {
                if food_pos == &*head_pos {
                    let new_segment =
                        spawn_segment(&mut commands, segment_material.0, last_position);
                    let mut segment = segments.get_mut::<SnakeSegment>(segment_entity).unwrap();
                    segment.next_segment = Some(new_segment);
                    commands.despawn(ent);
                }
            }
        }
    }
}

fn food_spawner(
    mut commands: Commands,
    food_material: Res<FoodMaterial>,
    time: Res<Time>,
    mut timer: ResMut<FoodSpawnTimer>,
) {
    timer.0.tick(time.delta_seconds);
    if timer.0.finished {
        commands
            .spawn(SpriteComponents {
                material: food_material.0,
                ..Default::default()
            })
            .with(Food)
            .with(Position {
                x: (random::<f32>() * ARENA_WIDTH as f32) as i32,
                y: (random::<f32>() * ARENA_HEIGHT as f32) as i32,
            })
            .with(Size::square(0.8));
    }
}

fn game_over_system(
    mut commands: Commands,
    mut reader: Local<EventReader<GameOverEvent>>,
    game_over_events: Res<Events<GameOverEvent>>,
    segment_material: Res<SegmentMaterial>,
    head_material: Res<HeadMaterial>,
    mut segments: Query<(Entity, &SnakeSegment)>,
    mut food: Query<(Entity, &Food)>,
    mut heads: Query<(Entity, &SnakeHead)>,
) {
    if reader.iter(&game_over_events).next().is_some() {
        for (ent, _segment) in &mut segments.iter() {
            commands.despawn(ent);
        }
        for (ent, _food) in &mut food.iter() {
            commands.despawn(ent);
        }
        for (ent, _head) in &mut heads.iter() {
            commands.despawn(ent);
        }
        spawn_initial_snake(&mut commands, head_material, segment_material);
    }
}

fn size_scaling(windows: Res<Windows>, mut q: Query<(&Size, &mut Sprite)>) {
    for (size, mut sprite) in &mut q.iter() {
        let window = windows.get_primary().unwrap();
        sprite.size = Vec2::new(
            size.width * window.width as f32 / ARENA_WIDTH as f32,
            size.height * window.height as f32 / ARENA_HEIGHT as f32,
        );
    }
}

fn position_translation(windows: Res<Windows>, mut q: Query<(&Position, &mut Transform)>) {
    fn convert(p: f32, bound_window: f32, bound_game: f32) -> f32 {
        p / bound_game * bound_window - (bound_window / 2.0)
    }
    let window = windows.get_primary().unwrap();
    for (pos, mut transform) in &mut q.iter() {
        transform.set_translation(Vec3::new(
            convert(pos.x as f32, window.width as f32, ARENA_WIDTH as f32),
            convert(pos.y as f32, window.height as f32, ARENA_HEIGHT as f32),
            0.0,
        ));
    }
}

fn main() {
    App::build()
        .add_resource(WindowDescriptor {
            title: "Snake!".to_string(),
            width: 1000,
            height: 1000,
            ..Default::default()
        })
        .add_resource(ClearColor(Color::rgb(0.04, 0.04, 0.04)))
        .add_resource(SnakeMoveTimer(Timer::new(Duration::from_millis(150), true)))
        .add_resource(FoodSpawnTimer(Timer::new(
            Duration::from_millis(1000),
            true,
        )))
        .add_event::<GameOverEvent>()
        .add_startup_system(setup.system())
        .add_startup_stage("game_setup")
        .add_startup_system_to_stage("game_setup", game_setup.system())
        .add_system(snake_movement.system())
        .add_system(food_spawner.system())
        .add_system(game_over_system.system())
        .add_system(position_translation.system())
        .add_system(size_scaling.system())
        .add_default_plugins()
        .run()
}
