use alloc::{boxed::Box, rc::Rc, vec, vec::Vec};
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::{
    structures::paging::{
        PageTable, PageTableFlags, PageTableIndex, PhysFrame, RecursivePageTable,
    },
    PhysAddr,
};

lazy_static! {
    pub static ref SCHEDULER: Mutex<Scheduler> = { Mutex::new(Scheduler::new()) };
}

#[derive(Default)]
pub struct Registers {
    eax: u64,
    ebx: u64,
    ecx: u64,
    edx: u64,
    esi: u64,
    edi: u64,
    esp: u64,
    ebp: u64,
    eip: u64,
    eflags: u64,
    cr3: u64,
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

    pub fn setup(&mut self) {
        // Load first module into memory (already done by the bootloader)
        let mut new_task = Task::new();

        // setup all pages
        let mut page_table4 = Box::new(PageTable::new());
        // let mut page_table5 = Box::new(PageTable::new());

        let pointer: *mut PageTable = &mut *page_table4;

        // unsafe {
        //     core::ptr::copy(
        //         super::memory::P4,
        //         pointer,
        //         core::mem::size_of::<PageTable>(),
        //         // core::mem::size_of::<PageTable>(),
        //     );
        // };

        // page_table4[511].set_addr(
        //     PhysAddr::new(pointer as u64),
        //     PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
        // );

        new_task.registers.cr3 = pointer as u64;

        self.tasks.push(new_task);

        let phys_p4 = if let Some(mapper) = &*super::allocator::MAPPER.lock() {
            use x86_64::{structures::paging::MapperAllSizes, VirtAddr};

            mapper
                .translate_addr(VirtAddr::new(super::memory::P4 as u64))
                .unwrap()
        } else {
            panic!("WTF");
        };

        unsafe {
            x86_64::registers::control::Cr3::write(
                PhysFrame::containing_address(phys_p4),
                x86_64::registers::control::Cr3Flags::empty(),
            );
        };

        // create task
        // create new PageDir
        // Load task registers (Cr3 and EIP)
        // push task in scheduler tasks
        // push arguments on the stack
        // switch to userland
        // --> on schedule interrupt
        // task.next() et basta
    }
}

pub fn init() {
    SCHEDULER.lock().setup();
}
