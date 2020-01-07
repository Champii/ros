use alloc::{boxed::Box, rc::Rc, vec, vec::Vec};
use lazy_static::lazy_static;
use spin::Mutex;

lazy_static! {
    pub static ref SCHEDULER: Mutex<Scheduler> = { Mutex::new(Scheduler::new()) };
}

#[derive(Default)]
pub struct Registers {
    eax: u32,
    ebx: u32,
    ecx: u32,
    edx: u32,
    esi: u32,
    edi: u32,
    esp: u32,
    ebp: u32,
    eip: u32,
    eflags: u32,
    cr3: u32,
}

impl Registers {
    pub fn new() -> Self {
        Self::default()
    }
}

pub struct Task {
    registers: Registers,
}

impl Task {
    pub fn new() -> Self {
        Self {
            registers: Registers::new(),
        }
    }
}

pub struct Scheduler {
    tasks: Vec<Task>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self { tasks: vec![] }
    }

    pub fn setup(&self) {}
}

pub fn init() {
    SCHEDULER.lock().setup();
}
