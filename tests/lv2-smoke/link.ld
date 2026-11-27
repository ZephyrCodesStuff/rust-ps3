OUTPUT_FORMAT("elf64-powerpc")
OUTPUT_ARCH(powerpc:common64)
ENTRY(_start)
PHDRS {
    text PT_LOAD FLAGS(0x00400005);
    data PT_LOAD FLAGS(0x00600006);
    proc 0x60000001 FLAGS(0);
    imports 0x60000002 FLAGS(0);
}
SECTIONS {
    . = 0x10000;
    .text : { *(.text .text.* .sceStub.text) } :text
    .rodata : { *(.entrydesc .rodata .rodata.*) } :text
    . = ALIGN(0x10000);
    .opd : { *(.opd) } :data
    .got : { *(.got .toc) } :data
    __toc = ADDR(.got) + 0x8000;
    .data : { *(.data*) } :data
    .lib.stub : { KEEP(*(.lib.stub)) } :data
    .sys_proc_param : { KEEP(*(.sys_proc_param)) } :data :proc
    .sys_proc_prx_param : { KEEP(*(.sys_proc_prx_param)) } :data :imports
    .bss : { *(.bss*) } :data
    /DISCARD/ : { *(.eh_frame .note.GNU-stack) }
}
