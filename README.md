# Zygisk API bindings for Rust

## Compatibility
| API Version | Minimum Magisk Version | Minimum ZygiskNext Version               | Implemented |
| ----------- | ---------------------- | ---------------------------------------- | ----------- |
| v1          | 23014                  | v4-0.1.0                                 | ✅           |
| v2          | 23019                  | v4-0.1.0                                 | ✅           |
| v3          | 24300                  | v4-0.1.0                                 | ✅           |
| v4          | 25204                  | v4-0.1.0                                 | ✅           |
| v5          | 26403                  | ~~It's supported in the latest version~~ | ✅           |


## References
- Zygisk API
  - v1 C++ header: https://github.com/topjohnwu/Magisk/blob/b8c158828484e27e2e7d6d7cb5803e6af270dc49/native/jni/zygisk/api.hpp
  - v2 C++ header: https://github.com/topjohnwu/Magisk/blob/06531f6d06a73b4770762964e41201b9f157923b/native/jni/zygisk/api.hpp
  - v3 C++ header: https://github.com/topjohnwu/Magisk/blob/1565bf5442e10b0f1b1908856f21e45703baa29a/native/src/zygisk/api.hpp
  - v4 C++ header: https://github.com/topjohnwu/Magisk/blob/65c18f9c09afa80774867b6ef26622ed7b4e0c96/native/src/core/zygisk/api.hpp
  - v5 C++ header: https://github.com/topjohnwu/Magisk/blob/e35925d520b5fab3acc96c1f137f951edca06760/native/src/core/zygisk/api.hpp