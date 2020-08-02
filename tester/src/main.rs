use minifb::{Key, Window, WindowOptions, KeyRepeat};
use minifb::Scale::X4;

use engine::{Model, Msg};
use engine::display::{OAM, HEIGHT, WIDTH};

fn draw(oam: &OAM, window: &mut Window) {
    let mut buffer = [u32::MAX; WIDTH * HEIGHT];

    for row in 0..HEIGHT {
        for col in 0..WIDTH {
            for object in &oam.objects {
                if col >= object.x as usize &&
                    col < object.x as usize + object.width as usize &&
                    row >= object.y as usize &&
                    row < object.y as usize + object.height as usize
                {
                    let byte = object.sprite[(col - object.x as usize) / 8 + ((row - object.y as usize) * (object.width as usize / 8))];
                    let value = byte & (1 << (7 - ((col - object.x as usize) % 8) as u8));

                    if value > 0 {
                        buffer[col + row * WIDTH] = 0;
                        break;
                    }
                }
            }
        }
    }

    window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
}

fn main() {
    let mut options = WindowOptions::default();
    options.scale = X4;

    let mut window = Window::new(
        "foss-walker",
        WIDTH,
        HEIGHT,
        options,
    ).unwrap_or_else(|e| {
        panic!("{}", e);
    });

    window.limit_update_rate(Some(std::time::Duration::from_micros(33200)));

    let mut model = Model::new();

    while window.is_open() & &!window.is_key_down(Key::Escape) {
        if window.is_key_pressed(Key::Space, KeyRepeat::No) {
            model.update(Msg::Pressed);
        }
        model.update(Msg::Tick);
        draw(&model.view(), &mut window);
    }
}