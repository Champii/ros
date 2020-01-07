use super::serial_println;
use lazy_static::lazy_static;
use x86_64::{
    structures::{
        gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector},
        tss::TaskStateSegment,
    },
    VirtAddr,
};

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

lazy_static! {
    static ref TSS: TaskStateSegment = {
        serial_println!("   Create TSS:");

        let mut tss = TaskStateSegment::new();

        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = 4096;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            serial_println!("       Set kernel stack: Size: {:#?}", STACK_SIZE);

            let stack_start = VirtAddr::from_ptr(unsafe { &STACK });
            let stack_end = stack_start + STACK_SIZE;

            stack_end
        };
        tss
    };
}

lazy_static! {
    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        serial_println!("   Create GDT");

        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
        (
            gdt,
            Selectors {
                code_selector,
                tss_selector,
            },
        )
    };
}

struct Selectors {
    code_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}

pub fn init() {
    serial_println!("Init GDT:");

    use x86_64::instructions::segmentation::set_cs;
    use x86_64::instructions::tables::load_tss;

    serial_println!("   Load GDT: {:#?}", GDT.0);

    GDT.0.load();

    unsafe {
        serial_println!("   Set CS: {:#?}", GDT.1.code_selector);
        set_cs(GDT.1.code_selector);

        serial_println!("   Set TSS: {:#?}", GDT.1.tss_selector);
        load_tss(GDT.1.tss_selector);
    }
}
