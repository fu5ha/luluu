MEMORY {
    BOOT2 : ORIGIN = 0x10000000, LENGTH = 0x100
    FLASH : ORIGIN = 0x10000100, LENGTH = 2M - 0x100
    /* NVM   : ORIGIN = 0x10200000, LENGTH = 16M - 2M */
    RAM   : ORIGIN = 0x20000000, LENGTH = 256K
    SRAM4 : ORIGIN = 0x20040000, LENGTH = 4K
    SRAM5 : ORIGIN = 0x20080000, LENGTH = 4K
}

EXTERN(BOOT2_FIRMWARE)

SECTIONS {
    /* ### Boot loader */
    .boot2 ORIGIN(BOOT2) :
    {
        KEEP(*(.boot2));
    } > BOOT2
} INSERT BEFORE .text;

SECTIONS {
    .sram4 ORIGIN(SRAM4) :
    {
        KEEP(*(.sram4));
    } > SRAM4

    .sram5 ORIGIN(SRAM5) :
    {
        KEEP(*(.sram5));
    } > SRAM5
}