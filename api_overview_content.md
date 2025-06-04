## API Version Overview

This section details the evolution of the Zygisk API as exposed by this crate, highlighting key features and changes in each version. The crate maps these Zygisk C API versions to corresponding Rust modules (e.g., `zygisk_api::api::V1`, `zygisk_api::raw::v1`) that implement the `ZygiskRaw` trait.

### API Version 1 (V1)

Version 1 establishes the baseline for Zygisk module interaction. It provides the foundational capabilities for modules to load and interact with Zygote and app specialization.

*   **Key Functionalities & APIs:**
    *   **Module Registration:** The fundamental ability for a module to register itself with Zygisk, providing callbacks for lifecycle events. This is handled by `register_module!` and the underlying `ZygiskRaw` implementation for V1.
    *   **Access to Module Directory:** `api.get_module_dir()` (exposed via `BaseApi` in `raw::v1::ApiTable` and directly on `ZygiskApi<V1>`) provides a file descriptor to the module's private data directory. This is essential for modules to store and access their own files, configurations, or additional native libraries.
    *   **Set Zygisk Options:** `api.set_option(option: ZygiskOption)` allows modules to influence Zygisk's behavior concerning the module itself. Common options (as defined in `raw::v1::transparent::ZygiskOption`) include:
        *   `ForceDenylistUnmount`: Instructs Zygisk to forcibly unmount any filesystems mounted by the module when operating in processes that are on Magisk's denylist. This helps ensure module features do not affect explicitly excluded applications.
        *   `DlCloseModuleLibrary`: Tells Zygisk to `dlclose` (unload) the module's shared library from memory after the initial loading and registration phase. This can reduce the module's memory footprint but means the module must manage its state carefully if it needs to persist across Zygisk invocations (e.g., by storing function pointers or state in static variables or leveraging the companion process).
    *   **JNI Native Method Hooking (Raw Capability):** The `raw::v1::ApiTable` includes `hook_jni_native_methods_fn`. This function pointer indicates Zygisk v1's underlying capability to allow modules to intercept and replace implementations of Java `native` methods. This is a powerful tool for modifying interactions between Java code and native libraries. The `zygisk-api` crate exposes this via `api.hook_jni_native_methods()`.
    *   **PLT Hooking (Raw Capability, Regex-based):** The `raw::v1::ApiTable` includes `plt_hook_register_fn`, `plt_hook_exclude_fn`, and `plt_hook_commit_fn`. These enable modules to hook functions in the Procedure Linkage Table (PLT) of ELF binaries. This allows interception of calls to functions within native libraries. In V1, the target ELFs for `plt_hook_register_fn` are typically identified by a **regex string** matching their path.
    *   **Companion Process Communication (Raw Capability):** The `raw::v1::ApiTable` includes `connect_companion_fn`, allowing the module to establish a Unix domain socket connection with its dedicated companion process. This is crucial for tasks requiring different privilege levels (e.g., root access) or for offloading operations.

*   **Specialization Arguments:**
    *   **`AppSpecializeArgs`** (from `raw::v1::transparent::AppSpecializeArgs`): This struct provides information to the `pre_app_specialize` and `post_app_specialize` callbacks.
        *   *Key Required Fields:* `uid`, `gid`, `gids` (groups), `runtime_flags`, `mount_external` (related to external storage visibility), `se_info` (SELinux context string), `nice_name` (process name string), `instruction_set` (CPU architecture), `app_data_dir` (application's data directory).
        *   *Key Optional Fields (pointers, must be checked for null if accessed directly at the FFI layer):* `is_child_zygote`, `is_top_app`, `pkg_data_info_list` (detailed package data), `whitelisted_data_info_list`, `mount_data_dirs` (boolean flag to control data directory mounting), `mount_storage_dirs` (boolean flag for storage directory mounting). These optional fields provide more granular control and context about the application being specialized.
    *   **`ServerSpecializeArgs`** (from `raw::v1::transparent::ServerSpecializeArgs`): Passed to system server specialization callbacks.
        *   Includes: `uid`, `gid`, `gids`, `runtime_flags`, `permitted_capabilities`, `effective_capabilities`.

### API Version 2 (V2)

Version 2 builds upon V1 by making some functionalities more explicit in the API table and introducing Zygisk state flags.

*   **Key Functionalities & APIs:**
    *   **Get Module Directory FD (Explicit):** While accessible in V1, `get_module_dir_fn` is now an explicit function pointer in `raw::v2::ApiTable`. The safe wrapper `api.get_module_dir()` uses this.
    *   **Connect to Companion (Explicit):** `connect_companion_fn` remains, now formally part of the V2 table structure if it wasn't considered part of `BaseApi` by some interpretations.
    *   **Get Zygisk Flags:** `api.get_flags()` (from `get_flags_fn` in `raw::v2::ApiTable`) is introduced. This function returns `StateFlags` (defined in `raw::v2::transparent`), which can include:
        *   `PROCESS_GRANTED_ROOT`: Indicates if the current process is running with root (UID 0) privileges.
        *   `PROCESS_ON_DENYLIST`: Indicates if the current process is on Magisk's denylist. Modules should typically check this flag and avoid performing modifications in denylisted processes to respect user choice.
    *   All other core functionalities from V1 (JNI/PLT hooking, setting options) are retained with the same signatures in `raw::v2::ApiTable`.

*   **Specialization Arguments:**
    *   `AppSpecializeArgs` and `ServerSpecializeArgs` are typically reused directly from V1's definitions (e.g., `raw::v2::transparent` re-exports `raw::v1::transparent::AppSpecializeArgs`). No structural changes to these arguments are introduced at this API level by the Zygisk C API itself.

### API Version 3 (V3)

Version 3 primarily focuses on enhancing application specialization arguments, offering more control over the forking process.

*   **Key Functionalities & APIs:**
    *   The raw Zygisk function table (`raw::v3::ApiTable`) is structurally identical to V2's. Thus, core Zygisk functions like companion connection, JNI/PLT hooking, getting flags, etc., remain available with the same signatures.
    *   **Exempt File Descriptor (Raw Capability):** Although not a new function pointer in the `ApiTable` compared to some interpretations of earlier generic tables, the concept of exempting file descriptors becomes more formalized. The `exempt_fd_fn` function pointer (if considered distinct or newly emphasized) allows a module to request Zygote to keep a specific file descriptor open during app specialization. This is crucial if the module has opened FDs in Zygote that need to be passed into the newly spawned app process. (Note: `exempt_fd_fn` is present in `raw::v4::ApiTable` and `raw::v5::ApiTable`; its presence in V3 depends on the exact Zygisk source interpretation this crate targets for V3. The `AppSpecializeArgs` for V3 does get `fds_to_ignore`).

*   **Specialization Arguments:**
    *   **`AppSpecializeArgs`** (from `raw::v3::transparent::AppSpecializeArgs`):
        *   Adds **`rlimits`** (type `jobjectArray`): This new *required* field provides the module with access to the target application's resource limits (rlimits) as an array of Java `android.os.strictmode.BlockGuardPolicy.Violation` (or similar) objects. This allows modules to inspect or potentially request modifications to these limits before the app fully starts.
        *   Adds optional **`fds_to_ignore`** (type `jintArray`): Allows the module to provide an array of file descriptors that Zygote should *not* close when forking the app process. This is useful if the module has, for example, opened sockets or pipes in Zygote that need to be inherited by the child app process.
    *   `ServerSpecializeArgs` are reused from V1's definitions (via `raw::v3::transparent`).

### API Version 4 (V4)

Version 4 introduces a significant refinement to PLT hooking, making it more precise by using device and inode numbers instead of regex matching for ELFs.

*   **Key Functionalities & APIs:**
    *   **PLT Hooking (Device/Inode-based):** The `plt_hook_register_fn` in `raw::v4::ApiTable` changes its signature. It now takes `dev_t device` and `ino_t inode` as arguments to identify the target ELF file, along with the `symbol` name and replacement function. This is more robust than path-based regex matching as it uniquely identifies a file regardless of its mount point or if it's accessed via symlinks. The safe `api.plt_hook_register()` wrapper reflects this.
    *   **Exempt File Descriptor (Explicit):** `exempt_fd_fn` is explicitly part of `raw::v4::ApiTable`. This function allows a module to tell Zygote not to close a specific file descriptor when an app process is forked. This is a more granular way to pass FDs compared to the `fds_to_ignore` array.
    *   Other core functionalities (companion connection, JNI hooking, setting options, getting flags) are retained.

*   **Specialization Arguments:**
    *   `AppSpecializeArgs` structure itself is typically reused from V3's definition (e.g., `raw::v4::transparent` re-exports `raw::v3::transparent::AppSpecializeArgs`). However, Zygisk v4 might more reliably populate or expect modules to use the optional fields introduced in V1/V3, such as `is_top_app`, `pkg_data_info_list`, `whitelisted_data_info_list`, `mount_data_dirs`, and `mount_storage_dirs`. These fields provide more context about the app (e.g., if it's a foreground app) and allow finer control over how its data directories are mounted.
    *   `ServerSpecializeArgs` structure is reused from V1's definition (via `raw::v4::transparent`).

### API Version 5 (V5)

Version 5 further enhances app specialization by adding the ability to mount system property overrides.

*   **Key Functionalities & APIs:**
    *   The raw Zygisk function table (`raw::v5::ApiTable`) is structurally identical to V4's. Core Zygisk functions like companion connection, JNI hooking, PLT hooking (with dev/inode targeting), and `exempt_fd` remain the same.
    *   **Hook JNI Native Methods (Explicit):** `hook_jni_native_methods_fn` remains a key feature, explicitly part of the V5 table structure if it wasn't considered part of a base table before.

*   **Specialization Arguments:**
    *   **`AppSpecializeArgs`** (from `raw::v5::transparent::AppSpecializeArgs`):
        *   Adds optional **`mount_sysprop_overrides`** (type `jboolean`): This new field allows a module to request Zygisk to mount system property overrides specifically for the application being specialized. If a module packages a `system.prop` file (or provides properties through other means that Zygisk recognizes), setting this flag to true would apply those properties only to the app being launched. This enables powerful per-app customization of system behavior as perceived by that app, without affecting other apps or the system globally.
    *   `ServerSpecializeArgs` are reused from V1's definitions (via `raw::v5::transparent`).

This overview illustrates the progressive enhancement of the Zygisk API, with each version offering more refined tools and control for module developers. The `zygisk-api` crate's design using `ZygiskRaw` and generic programming allows modules to target these specific versions in a type-safe manner.
