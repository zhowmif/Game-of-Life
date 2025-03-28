use bevy::ecs::system::SystemId;
use bevy::input::mouse::MouseWheel;
use bevy::window::PrimaryWindow;
use bevy::{prelude::*, window::WindowMode};
use std::collections::HashSet;

const GAME_TICK_SECOND: f32 = 0.1;

const NUM_OF_COLS: i32 = 100;
const NUM_OF_ROWS: i32 = 100;

const SQUARE_HEIGHT: f32 = 50.;
const SQUARE_WIDTH: f32 = 50.;
const CAMERA_MOVE_FACTOR: f32 = 0.3;

#[derive(Resource)]
struct GameTickTimer(Timer);

#[derive(Resource)]
struct OneShotSystems {
    game_logic: SystemId,
    render: SystemId,
}

impl FromWorld for OneShotSystems {
    fn from_world(world: &mut World) -> Self {
        OneShotSystems {
            game_logic: world.register_system(game_logic),
            render: world.register_system(handle_rendering),
        }
    }
}

#[derive(Component)]
struct Square {
    x: i32,
    y: i32,
}

#[derive(Component)]
#[component(storage = "SparseSet")]
struct Alive;

#[derive(States, Debug, Clone, PartialEq, Eq, Hash)]
enum GameState {
    Placing,
    Ongoing,
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
struct PlacingSet;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
struct OngoingSet;

#[derive(Eq, Hash, PartialEq, Clone)]
struct SquareIdentifier {
    entity: Entity,
    number_of_neighbors: u32,
}

#[derive(Resource)]
struct SquareMap {
    map: Vec<Vec<SquareIdentifier>>,
}

#[derive(Resource)]
enum LogicState {
    CalculationNeeded,
    CalculatingCurrently,
    Calculated,
}

#[derive(Resource)]
struct RenderInput {
    entities_that_died: Vec<Entity>,
    entities_born: Vec<Entity>,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resizable: false,
                mode: WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
                ..default()
            }),
            ..default()
        }))
        .insert_state(GameState::Placing)
        .configure_sets(Update, PlacingSet.run_if(in_state(GameState::Placing)))
        .configure_sets(Update, OngoingSet.run_if(in_state(GameState::Ongoing)))
        .insert_resource(SquareMap {
            map: Vec::with_capacity(SQUARE_HEIGHT as usize),
        })
        .insert_resource(RenderInput {
            entities_that_died: Vec::new(),
            entities_born: Vec::new(),
        })
        .insert_resource(LogicState::CalculationNeeded)
        .insert_resource(GameTickTimer(Timer::from_seconds(
            GAME_TICK_SECOND,
            TimerMode::Repeating,
        )))
        .init_resource::<OneShotSystems>()
        .add_systems(Startup, setup)
        .add_systems(Update, (handle_scroll, handle_move, handle_state_change))
        .add_systems(Update, handle_click.in_set(PlacingSet))
        .add_systems(Update, game_loop.in_set(OngoingSet))
        .run();
}

fn setup(mut commands: Commands, mut square_map: ResMut<SquareMap>) {
    commands.spawn(Camera2d);

    for row in 0..NUM_OF_ROWS {
        square_map
            .map
            .push(Vec::with_capacity(SQUARE_WIDTH as usize));

        for col in 0..NUM_OF_COLS {
            let square_id = commands
                .spawn((
                    Square { x: col, y: row },
                    Sprite {
                        color: Color::WHITE,
                        custom_size: Some(Vec2::new(SQUARE_WIDTH, SQUARE_HEIGHT)),
                        ..default()
                    },
                    Transform {
                        translation: Vec2::new(
                            (-NUM_OF_COLS as f32 * SQUARE_WIDTH / 2.) + col as f32 * SQUARE_WIDTH,
                            (NUM_OF_ROWS as f32 * SQUARE_HEIGHT / 2.) - row as f32 * SQUARE_HEIGHT,
                        )
                        .extend(0.),
                        ..default()
                    },
                ))
                .id();

            square_map.map.last_mut().unwrap().push(SquareIdentifier {
                entity: square_id,
                number_of_neighbors: 0,
            })
        }
    }
}

fn handle_scroll(
    mut evr_scroll: EventReader<MouseWheel>,
    mut query_camera: Query<&mut OrthographicProjection, With<Camera2d>>,
) {
    let mut camera_projection = query_camera.single_mut();

    for ev in evr_scroll.read() {
        camera_projection.scale /= if ev.y.is_sign_positive() { 1.1 } else { 0.9 };
    }
}

fn handle_state_change(
    keys: Res<ButtonInput<KeyCode>>,
    game_state: Res<State<GameState>>,
    mut next_game_state: ResMut<NextState<GameState>>,
) {
    if keys.just_pressed(KeyCode::Space) {
        match game_state.get() {
            GameState::Placing => next_game_state.set(GameState::Ongoing),
            GameState::Ongoing => next_game_state.set(GameState::Placing),
        }
    }
}

fn handle_move(
    keys: Res<ButtonInput<KeyCode>>,
    mut query_camera: Query<&mut Transform, With<Camera2d>>,
) {
    let mut camera_transform = query_camera.single_mut();
    let mut move_x = 0.;
    let mut move_y = 0.;

    if keys.pressed(KeyCode::KeyD) {
        move_x += SQUARE_WIDTH * CAMERA_MOVE_FACTOR;
    }

    if keys.pressed(KeyCode::KeyA) {
        move_x -= SQUARE_WIDTH * CAMERA_MOVE_FACTOR;
    }

    if keys.pressed(KeyCode::KeyW) {
        move_y += SQUARE_HEIGHT * CAMERA_MOVE_FACTOR;
    }

    if keys.pressed(KeyCode::KeyS) {
        move_y -= SQUARE_HEIGHT * CAMERA_MOVE_FACTOR;
    }

    camera_transform.translation.x += move_x;
    camera_transform.translation.y += move_y;
}

fn handle_click(
    mouse_button: Res<ButtonInput<MouseButton>>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    square_map: Res<SquareMap>,
    mut q_square_sprite: Query<&mut Sprite, With<Square>>,
    mut commands: Commands,
) {
    if !mouse_button.pressed(MouseButton::Left) {
        return;
    }

    let (camera, camera_transform) = q_camera.single();

    if let Some(position) = q_window
        .single()
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor).ok())
        .map(|ray| ray.origin.truncate())
    {
        let square_x = ((position.x
            - (-NUM_OF_COLS as f32 * SQUARE_WIDTH / 2. - SQUARE_WIDTH / 2.))
            / SQUARE_WIDTH)
            .floor();
        let square_y = (((NUM_OF_ROWS as f32 * SQUARE_HEIGHT / 2. + SQUARE_HEIGHT / 2.)
            - position.y)
            / SQUARE_HEIGHT)
            .floor();

        if square_x < 0. || square_y < 0. {
            return;
        }

        if let Some(square_id) = square_map
            .map
            .get(square_y as usize)
            .map(|row| row.get(square_x as usize))
            .flatten()
        {
            let mut square_sprite = q_square_sprite.get_mut(square_id.entity).unwrap();
            square_sprite.color = Color::BLACK;
            commands.entity(square_id.entity).insert(Alive);
        }
    }
}

fn game_loop(
    mut logic_state: ResMut<LogicState>,
    mut commands: Commands,
    one_shot_systems: Res<OneShotSystems>,
    mut game_tick_timer: ResMut<GameTickTimer>,
    time: Res<Time>,
) {
    match *logic_state {
        LogicState::CalculationNeeded => {
            *logic_state = LogicState::CalculatingCurrently;
            commands.run_system(one_shot_systems.game_logic)
        }
        LogicState::CalculatingCurrently => {}
        LogicState::Calculated => {
            if !game_tick_timer.0.tick(time.delta()).just_finished() {
                return;
            }

            commands.run_system(one_shot_systems.render);
        }
    }
}

fn game_logic(
    mut square_map: ResMut<SquareMap>,
    q_alive_squares: Query<&Square, With<Alive>>,
    mut logic_state: ResMut<LogicState>,
    mut render_input: ResMut<RenderInput>,
) {
    render_input.entities_born.clear();
    render_input.entities_that_died.clear();
    let mut potentially_changed_squares: HashSet<(usize, usize)> = HashSet::new();

    for square in q_alive_squares.iter() {
        potentially_changed_squares.insert((square.y as usize, square.x as usize));

        for dx in -1..=1 {
            for dy in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }

                let y = square.y + dy;
                let x = square.x + dx;

                if x < 0 || y < 0 || x >= NUM_OF_COLS || y >= NUM_OF_ROWS {
                    continue;
                }

                potentially_changed_squares.insert((y as usize, x as usize));

                if let Some(neighbor) = square_map
                    .map
                    .get_mut(y as usize)
                    .map(|row| row.get_mut(x as usize))
                    .flatten()
                {
                    neighbor.number_of_neighbors += 1;
                }
            }
        }
    }

    for (y, x) in potentially_changed_squares.into_iter() {
        if let Some(square) = square_map
            .map
            .get_mut(y)
            .map(|row| row.get_mut(x))
            .flatten()
        {
            let was_previously_alive = q_alive_squares.get(square.entity).is_ok();
            //println!(
            //    "checking square ({},{}), alive - {}, neighbors - {}",
            //    y, x, was_previously_alive, square.number_of_neighbors
            //);

            if was_previously_alive
                && square.number_of_neighbors != 2
                && square.number_of_neighbors != 3
            {
                render_input.entities_that_died.push(square.entity);
            } else if square.number_of_neighbors == 3 {
                render_input.entities_born.push(square.entity);
            }

            square.number_of_neighbors = 0;
        }
    }

    *logic_state = LogicState::Calculated;
}

fn handle_rendering(
    render_input: Res<RenderInput>,
    mut commands: Commands,
    mut q_squares: Query<&mut Sprite>,
    mut logic_state: ResMut<LogicState>,
) {
    for dead_square in render_input.entities_that_died.iter() {
        let mut entity = q_squares.get_mut(*dead_square).unwrap();
        entity.color = Color::WHITE;
        commands.get_entity(*dead_square).unwrap().remove::<Alive>();
    }

    for square_born in render_input.entities_born.iter() {
        let mut entity = q_squares.get_mut(*square_born).unwrap();
        entity.color = Color::BLACK;
        commands.get_entity(*square_born).unwrap().insert(Alive);
    }

    *logic_state = LogicState::CalculationNeeded;
}
