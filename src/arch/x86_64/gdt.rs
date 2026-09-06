//! Global Descriptor Table (GDT) for x86_64.

use core::arch::asm;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GdtEntry {
    limit_low: u16,
    base_low: u16,
    base_middle: u8,
    access: u8,
    granularity: u8,
    base_high: u8,
}

impl GdtEntry {
    pub const NULL: Self = GdtEntry { limit_low: 0, base_low: 0, base_middle: 0, access: 0, granularity: 0, base_high: 0 };
    pub const KERNEL_CODE: Self = GdtEntry { limit_low: 0, base_low: 0, base_middle: 0, access: 0x9A, granularity: 0x20, base_high: 0 };
    pub const KERNEL_DATA: Self = GdtEntry { limit_low: 0, base_low: 0, base_middle: 0, access: 0x92, granularity: 0, base_high: 0 };
    pub const USER_CODE: Self = GdtEntry { limit_low: 0, base_low: 0, base_middle: 0, access: 0xFA, granularity: 0x20, base_high: 0 };
    pub const USER_DATA: Self = GdtEntry { limit_low: 0, base_low: 0, base_middle: 0, access: 0xF2, granularity: 0, base_high: 0 };
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct Tss {
    reserved0: u32,
    pub rsp0: u64,
    rsp1: u64,
    rsp2: u64,
    reserved1: u32,
    pub ist: [u64; 7],
    reserved2: u32,
    reserved3: u16,
    pub iomap_base: u16,
}

impl Tss {
    pub const fn new() -> Self {
        Tss {
            reserved0: 0, rsp0: 0, rsp1: 0, rsp2: 0, reserved1: 0,
            ist: [0; 7], reserved2: 0, reserved3: 0,
            iomap_base: core::mem::size_of::<Tss>() as u16,
        }
    }
}

#[repr(C)]
struct GdtPointer {
    limit: u16,
    base: u64,
}

pub const KERNEL_CODE_SELECTOR: u16 = 1 * 8;
pub const KERNEL_DATA_SELECTOR: u16 = 2 * 8;
pub const USER_CODE_SELECTOR: u16 = 3 * 8 | 3;
pub const USER_DATA_SELECTOR: u16 = 4 * 8 | 3;
pub const TSS_SELECTOR: u16 = 5 * 8;

static mut GDT: [GdtEntry; 7] = [
    GdtEntry::NULL, GdtEntry::KERNEL_CODE, GdtEntry::KERNEL_DATA,
    GdtEntry::USER_CODE, GdtEntry::USER_DATA,
    GdtEntry::NULL, GdtEntry::NULL,
];

static mut TSS: Tss = Tss::new();

pub unsafe fn init() {
    unsafe {
        // Set up the TSS descriptor.
        let tss_addr = core::ptr::addr_of!(TSS) as u64;
        let tss_limit = (core::mem::size_of::<Tss>() - 1) as u32;

        GDT[5] = GdtEntry {
            limit_low: (tss_limit & 0xFFFF) as u16,
            base_low: (tss_addr & 0xFFFF) as u16,
            base_middle: ((tss_addr >> 16) & 0xFF) as u8,
            access: 0x89,
            granularity: ((tss_limit >> 16) & 0x0F) as u8,
            base_high: ((tss_addr >> 24) & 0xFF) as u8,
        };
        GDT[6] = GdtEntry {
            limit_low: ((tss_addr >> 32) & 0xFFFF) as u16,
            base_low: ((tss_addr >> 48) & 0xFFFF) as u16,
            base_middle: 0, access: 0, granularity: 0, base_high: 0,
        };

        let gdtr = GdtPointer {
            limit: (7 * 8 - 1) as u16,
            base: core::ptr::addr_of!(GDT) as u64,
        };

        asm!(
            "lgdt [{}]",
            in(reg) &gdtr,
            options(nostack, preserves_flags),
        );

        asm!(
            "mov ax, {ds}",
            "mov ds, ax",
            "mov es, ax",
            "mov fs, ax",
            "mov gs, ax",
            "mov ss, ax",
            "mov ax, {cs}",
            "push rax",
            "lea rax, [rip + 2f]",
            "push rax",
            "retfq",
            "2:",
            ds = const KERNEL_DATA_SELECTOR,
            cs = const KERNEL_CODE_SELECTOR,
            options(preserves_flags),
        );

        asm!(
            "ltr ax",
            in("eax") TSS_SELECTOR as u32,
            options(nostack, preserves_flags),
        );
    }
}

pub unsafe fn set_kernel_stack(stack_top: u64) {
    unsafe {
        core::ptr::write_unaligned(core::ptr::addr_of_mut!(TSS.rsp0), stack_top);
    }
}
