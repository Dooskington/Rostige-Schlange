extern crate gl;
extern crate libc;
extern crate glutin;
extern crate image;
extern crate nalgebra;
extern crate alga;
extern crate rand;
extern crate time;

mod gfx;
mod input;

use rand::*;
use gfx::*;
use input::*;
use glutin::VirtualKeyCode;
use time::*;

pub const MAX_MOVE_FREQUENCY_MS: i64 = 30;
pub const BASE_MOVE_FREQUENCY_MS: i64 = 100;
pub const SPRITE_SNAKE: Sprite = Sprite { graphic: 1 as char, color: COLOR_WHITE };
pub const SPRITE_FOOD: Sprite = Sprite { graphic: '$', color: COLOR_GREEN };

#[derive(PartialEq)]
pub enum Direction {
    None,
    North,
    South,
    East,
    West
}

#[derive(PartialEq)]
pub enum GameState {
    Playing,
    GameOver
}

pub struct Snake {
    segments: Vec<Coordinates>,
    direction: Direction,
    move_frequency: Duration,
    last_move_time: Tm,
    has_moved: bool
}

impl Snake {
    pub fn new(position: Coordinates) -> Snake {
        let mut segments: Vec<Coordinates> = Vec::new();
        segments.push(position);

        Snake {
            segments,
            direction: Direction::None,
            move_frequency: time::Duration::milliseconds(100),
            last_move_time: time::now(),
            has_moved: false
        }
    }
}

fn grow_snake(snake: &mut Snake) {
    let head = snake.segments.first().unwrap().clone();
    snake.segments.push(head);
}

pub struct Game {
    snake: Snake,
    food_position: Option<Coordinates>,
    score: u32,
    state: GameState
}

impl Game {
    pub fn new() -> Game {
        let initial_snake_position: Coordinates = Coordinates::new(15, 8);

        Game {
            snake: Snake::new(initial_snake_position),
            food_position: None,
            score: 0,
            state: GameState::Playing
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct Coordinates {
    pub x: i32,
    pub y: i32
}

impl Coordinates {
    pub fn new(x: i32, y: i32) -> Coordinates {
        Coordinates {
            x,
            y
        }
    }
}

fn main() {
    let window_title: &str = "Rostige Schlange";
    let window_width: u32 = 30 * gfx::CELL_WIDTH;
    let window_height: u32 = 15 * gfx::CELL_HEIGHT;

    let mut window: Window = Window::new(window_title, window_width, window_height);
    let mut renderer: Renderer = Renderer::new(&window);
    let mut input_man: InputMan = InputMan::new();

    let mut game: Game = Game::new();
    reset_food(&mut game);

    let frame_time: Duration = time::Duration::milliseconds(16);
    let one_second: Duration = time::Duration::seconds(1);

    let mut last_frame_time: Tm = time::now();
    let mut frame_timer: Duration = time::Duration::zero();
    let mut fps_timer: Duration = time::Duration::zero();
    let mut fps_counter: u32 = 0;
    let mut fps: u32 = 0;

    loop {
        input::process_events(&mut window, &mut input_man);
        if window.is_close_requested {
            break;
        }

        let delta_time: Duration = time::now() - last_frame_time;
        last_frame_time = time::now();

        frame_timer = frame_timer + delta_time;
        if frame_timer >= frame_time {
            frame_timer = time::Duration::zero();

            update(&mut input_man, &mut game);

            gfx::clear(&mut renderer);

            render(&mut renderer, &mut game);

            gfx::render(&mut renderer);
            gfx::display(&window);

            last_frame_time = time::now();
            fps_counter += 1;

            input::update_input(&mut input_man);
        }

        fps_timer = fps_timer + delta_time;
        if fps_timer >= one_second {
            fps_timer = time::Duration::zero();
            fps = fps_counter;
            fps_counter = 0;
        }
    }
}

fn reset_food(game: &mut Game) {
    let mut rng = rand::thread_rng();
    let x: i32 = rng.gen_range(2, 28);
    let y: i32 = rng.gen_range(2, 12);

    game.food_position = Some(Coordinates::new(x, y));
}

fn reset_snake(snake: &mut Snake) {
    snake.segments.clear();
    snake.segments.push(Coordinates::new(15, 8));
}

fn reset_game(game: &mut Game) {
    reset_snake(&mut game.snake);
    game.score = 0;
    game.state = GameState::Playing;
}

fn collect_food(game: &mut Game) {
    calc_move_frequency(game);
    reset_food(game);
    grow_snake(&mut game.snake);
    game.score += 1;
}

fn calc_move_frequency(game: &mut Game) {
    let mut move_frequency_ms: i64 = BASE_MOVE_FREQUENCY_MS - f32::powf(game.score as f32, 1.4) as i64;
    move_frequency_ms = move_frequency_ms.max(MAX_MOVE_FREQUENCY_MS);
    game.snake.move_frequency = time::Duration::milliseconds(move_frequency_ms);
}

fn game_over(game: &mut Game) {
    game.food_position = None;
    game.snake.move_frequency = time::Duration::milliseconds(BASE_MOVE_FREQUENCY_MS);
    game.snake.direction = Direction::None;
    game.state = GameState::GameOver;
}

fn handle_collision(game: &mut Game) {
    let head: Coordinates = game.snake.segments.first().unwrap().clone();

    // Segment collisions
    if game.snake.has_moved && (game.snake.direction != Direction::None) {
        let segments_cloned: Vec<Coordinates> = game.snake.segments.clone();
        for i in 1..segments_cloned.len() {
            let segment: &Coordinates = &segments_cloned[i];
            if head == *segment {
                game_over(game);
            }
        }
    }

    // Wall collisions
    if head.x <= 0 || head.x >= 28 || head.y <= 0 || head.y >= 13 {
        game_over(game);
    }

    // Food collision
    if let Some(food_position) = game.food_position.clone() {
        if head == food_position {
            collect_food(game);
        }
    }
}

fn update(input_man: &InputMan, game: &mut Game) {
    if game.state == GameState::Playing {
        update_snake(input_man, game);
        handle_collision(game);
    } else if game.state == GameState::GameOver {
        if is_key_pressed(input_man, VirtualKeyCode::Space) {
            reset_game(game);
        }
    }
}

fn render(renderer: &mut Renderer, game: &mut Game) {
    render_snake(renderer, &game.snake);

    // Render food
    if let Some(ref food_position) = game.food_position {
        gfx::draw_cell(renderer, food_position.x, food_position.y, SPRITE_FOOD);
    }

    // Render score text
    gfx::draw_string(renderer, 1, 14, &format!("SCORE: {}", game.score));

    // Render main window border
    gfx::draw_box(renderer, 0, 0, 29, 14);

    if game.state == GameState::GameOver {
        gfx::draw_string(renderer, 1, 1, "Press SPACE to play again.");
    } else if game.snake.direction == Direction::None {
        gfx::draw_string(renderer, 1, 1, "Use the WASD keys to move.");
    }
}

fn update_snake(input_man: &InputMan, game: &mut Game) {
    let snake: &mut Snake = &mut game.snake;

    // Input
    if input::is_key_pressed(input_man, VirtualKeyCode::W) {
        snake.direction = Direction::North;
    }
    else if input::is_key_pressed(input_man, VirtualKeyCode::A) {
        snake.direction = Direction::West;
    }
    else if input::is_key_pressed(input_man, VirtualKeyCode::S) {
        snake.direction = Direction::South;
    }
    else if input::is_key_pressed(input_man, VirtualKeyCode::D) {
        snake.direction = Direction::East;
    }

    // Movement
    let current_time: Tm = time::now();
    if (current_time - snake.last_move_time) > snake.move_frequency {
        snake.last_move_time = current_time;

        if snake.direction != Direction::None {
            // Update segment positions in reverse order (from tail to head)
            let segments_cloned: Vec<Coordinates> = snake.segments.clone();
            for i in (1..snake.segments.len()).rev() {
                let next_segment: &Coordinates = &segments_cloned[i - 1];
                let segment: &mut Coordinates = &mut snake.segments[i];

                segment.x = next_segment.x;
                segment.y = next_segment.y;
            }
        }

        // Update head position
        match snake.direction {
            Direction::North => { snake.segments.first_mut().unwrap().y += 1 },
            Direction::South => { snake.segments.first_mut().unwrap().y -= 1 },
            Direction::East => { snake.segments.first_mut().unwrap().x += 1 },
            Direction::West => { snake.segments.first_mut().unwrap().x -= 1 }
            Direction::None => {}
        }

        snake.has_moved = true;
    } else {
        snake.has_moved = false;
    }
}

fn render_snake(renderer: &mut Renderer, snake: &Snake) {
    for segment in &snake.segments {
        gfx::draw_cell(renderer, segment.x, segment.y, SPRITE_SNAKE);
    }
}