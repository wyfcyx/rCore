use crate::consts::{KERNEL_OFFSET, MEMORY_END, MEMORY_OFFSET};
use crate::memory::{init_heap, MemorySet, FRAME_ALLOCATOR};
use core::mem;
use log::*;
use rcore_memory::PAGE_SIZE;
use riscv::asm::sfence_vma_all;
use riscv::register::{satp, sstatus, stval};

/// Initialize the memory management module
pub fn init(dtb: usize) {
    // allow user memory access
    unsafe {
        sstatus::set_sum();
    }
    // initialize heap and Frame allocator
    init_frame_allocator();
    init_heap();
    remap_the_kernel(dtb);
    heap_test();
}

pub fn init_other() {
    unsafe {
        sstatus::set_sum(); // Allow user memory access
        satp::write(SATP);
        sfence_vma_all();
    }
}

fn init_frame_allocator() {
    use bitmap_allocator::BitAlloc;
    use core::ops::Range;

    let mut ba = FRAME_ALLOCATOR.lock();
    let range = to_range(
        (end as usize) - KERNEL_OFFSET + MEMORY_OFFSET + PAGE_SIZE,
        MEMORY_END,
    );
    ba.insert(range);

    info!("frame allocator: init end");

    /// Transform memory area `[start, end)` to integer range for `FrameAllocator`
    fn to_range(start: usize, end: usize) -> Range<usize> {
        let page_start = (start - MEMORY_OFFSET) / PAGE_SIZE;
        let page_end = (end - MEMORY_OFFSET - 1) / PAGE_SIZE + 1;
        assert!(page_start < page_end, "illegal range for frame allocator");
        page_start..page_end
    }
}

/// Remap the kernel memory address with 4K page recorded in p1 page table
fn remap_the_kernel(_dtb: usize) {
    let ms = MemorySet::new();
    unsafe {
        ms.activate();
    }
    unsafe {
        SATP = ms.token();
    }
    mem::forget(ms);
    info!("remap kernel end");
}

// First core stores its SATP here.
// Other cores load it later.
static mut SATP: usize = 0;

pub unsafe fn clear_bss() {
    let start = sbss as usize;
    let end = ebss as usize;
    let step = core::mem::size_of::<usize>();
    for i in (start..end).step_by(step) {
        (i as *mut usize).write(0);
    }
}

// Symbols provided by linker script
#[allow(dead_code)]
extern "C" {
    fn stext();
    fn etext();
    fn sdata();
    fn edata();
    fn srodata();
    fn erodata();
    fn sbss();
    fn ebss();
    fn start();
    fn end();
    fn bootstack();
    fn bootstacktop();
}

pub fn get_page_fault_addr() -> usize {
    stval::read()
}

pub fn set_page_table(vmtoken: usize) {
    satp::write(vmtoken);
    unsafe { sfence_vma_all() }
}

fn heap_test() {
    use alloc::boxed::Box;
    use alloc::vec::Vec;
    let v = Box::new(10);
    assert_eq!(*v, 10);
    core::mem::drop(v);

    let mut vec = Vec::new();
    for i in 0..100 {
        vec.push(i);
    }
    assert_eq!(vec.len(), 100);
    for (i, val) in vec.iter().enumerate() {
        assert_eq!(i, *val);
    }
    info!("memory test passed");
}
