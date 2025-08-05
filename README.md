# sm4sh_lib
Rust libraries and tools for working with rendering related file formats for Smash 4 for the Wii U.

Python bindings for sm4sh_model are available with [sm4sh_model_py](https://github.com/ScanMountGoat/sm4sh_model_py). For working with models in Blender, see [sm4sh_blender](https://github.com/ScanMountGoat/sm4sh_blender).

## Formats
| Format | Magic | Extension | Description |
| --- | --- | --- | --- |
| [Jtb](https://github.com/ScanMountGoat/sm4sh_lib/blob/main/sm4sh_lib/src/jtb.rs) | | jtb | joint tables |
| [Mta](https://github.com/ScanMountGoat/sm4sh_lib/blob/main/sm4sh_lib/src/mta.rs) | MTA2, MTA3, MTA4 | mta | material animations | 
| [Nhb](https://github.com/ScanMountGoat/sm4sh_lib/blob/main/sm4sh_lib/src/nhb.rs) | NHB, BHN | nhb | helper bones |
| [Nsh](https://github.com/ScanMountGoat/sm4sh_lib/blob/main/sm4sh_lib/src/nsh.rs) | NSP3 | nsh | shaders | 
| [Nud](https://github.com/ScanMountGoat/sm4sh_lib/blob/main/sm4sh_lib/src/nud.rs) | NDP3 | nud | models | 
| [Nut](https://github.com/ScanMountGoat/sm4sh_lib/blob/main/sm4sh_lib/src/nut.rs) | NTP3, NTWU | nut | textures | 
| [Omo](https://github.com/ScanMountGoat/sm4sh_lib/blob/main/sm4sh_lib/src/omo.rs) | OMO | omo | animations | 
| [Pack](https://github.com/ScanMountGoat/sm4sh_lib/blob/main/sm4sh_lib/src/pack.rs) | PACK, KCAP | pac | file archives |
| [Sb](https://github.com/ScanMountGoat/sm4sh_lib/blob/main/sm4sh_lib/src/sb.rs) | SWB, BWS | sb | swing bones |
| [Vbn](https://github.com/ScanMountGoat/sm4sh_lib/blob/main/sm4sh_lib/src/vbn.rs) | VBN, NVB | vbn | skeletons |

## Credits
This project is based on previous development and reverse engineering work done for [Smash Forge](https://github.com/jam1garner/Smash-Forge).
