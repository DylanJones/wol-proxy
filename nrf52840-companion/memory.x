/* nRF52840 with Adafruit/Seeed UF2 bootloader + SoftDevice S140 v7
   Layout notes (1 MiB total flash):
   - App base (with SD v7):           0x00027000
   - Bootloader settings page:        0x000FF000 (reserve, DO NOT TOUCH)
   - MBR parameters page:             0x000FE000 (reserve, DO NOT TOUCH)
   - We reserve our own 4 KiB config: 0x000FD000 (safe app-owned page)

   Therefore, application FLASH length excludes three 4 KiB pages at the top
   (config + MBR params + bootloader settings):
     LENGTH = 0x100000 - 0x27000 - 3*0x1000 = 0x000D7000
*/
MEMORY
{
  FLASH : ORIGIN = 0x00027000, LENGTH = 0x000D7000
  CFG   : ORIGIN = 0x000FD000, LENGTH = 4K
  RAM   : ORIGIN = 0x20000000, LENGTH = 256K
}

