/* nRF52840 + Adafruit/Seeed UF2 bootloader with SoftDevice S140 v7
   Application FLASH starts at 0x27000. Total flash is 1 MiB (0x100000), so LENGTH = 0x100000 - 0x27000 = 0xD9000.
   RAM remains full 256 KiB for this simple app. Adjust if your bootloader/SD reserves RAM. */
MEMORY
{
  FLASH : ORIGIN = 0x00027000, LENGTH = 0x000D9000
  RAM   : ORIGIN = 0x20000000, LENGTH = 256K
}
