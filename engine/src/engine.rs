use crate::display::{OAM, Object};
use crate::sprites::BLOCK;
use alloc::vec::Vec;

#[derive(Copy, Clone, Eq, PartialEq)]
enum Block {
    Filled,
    Empty,
}

pub struct Model {
    grid: [Block; 6 * 7],
    snake: [Block; 6],
    tick: usize,
    rising: bool,
    finished: bool,
    y: usize,
    length: usize,
}

impl Model {
    pub fn new() -> Self {
        let mut snake = [Block::Empty; 6];
        let length = 3;
        for index in 0..length {
            snake[index] = Block::Filled;
        }

        Model {
            rising: false,
            grid: [Block::Empty; 6 * 7],
            snake,
            tick: 0,
            y: 0,
            length,
            finished: false,
        }
    }

    pub fn update(&mut self, msg: Msg) {
        match msg {
            Msg::Pressed => {
                self.rising = true;
            }
            Msg::Tick => {
                if self.finished {
                    if self.rising {
                        *self = Model::new();
                        return;
                    } else {
                        return;
                    }
                }

                self.tick += 1;

                if self.rising {
                    let grid = &mut self.grid;
                    let y = self.y;

                    for x in 0..6 {
                        if self.snake[x] == Block::Filled {
                            if y == 0 || grid[x + y * 6 - 6] == Block::Filled {
                                grid[x + y * 6] = Block::Filled;
                            } else {
                                self.length -= 1;16
                            }
                        }
                    }

                    self.y += 1;

                    if self.length == 0 || self.y == 7 {
                        self.y = 0;
                        self.rising = false;
                        self.finished = true;
                        self.snake = [Block::Empty; 6];
                        return;
                    } else {
                        self.snake = [Block::Empty; 6];

                        for index in 0..self.length {
                            self.snake[index] = Block::Filled;
                        }
                    }
                }

                self.rising = false;

                let mut space_reached = false;

                if self.tick % 8 == 0 {
                    for x in 0..=6 {
                        let x = x % 6;

                        if self.snake[x] == Block::Empty {
                            space_reached = true;
                            continue;
                        }

                        if space_reached {
                            if self.snake[x] == Block::Filled {
                                self.snake[x] = Block::Empty;
                                self.snake[(x + self.length) % 6] = Block::Filled;
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn view(&self) -> OAM {
        let mut objects = Vec::with_capacity(8 * 16);

        for row in 0..7 {
            for col in 0..6 {
                let value = self.grid[row * 6 + col];

                if value == Block::Filled {
                    let object = Object {
                        sprite: &BLOCK,
                        width: 24,
                        height: 24,
                        x: col as u16 * 24,
                        y: (6 - row) as u16 * 24,
                    };

                    objects.push(object)
                }
            }
        }

        for x in 0..6 {
            if self.snake[x] == Block::Filled {
                let object = Object {
                    sprite: &BLOCK,
                    width: 24,
                    height: 24,
                    x: x as u16 * 24,
                    y: (6 - self.y) as u16 * 24,
                };

                objects.push(object)
            }
        }

        return OAM {
            objects
        };
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Msg {
    Pressed,
    Tick,
}