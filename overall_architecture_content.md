## Overall Architecture

The `zygisk-api` crate is designed to provide ergonomic and safe Rust bindings for the Zygisk API, a feature within Magisk that allows developers to run custom code within the Zygote process and every Android application process.

### Purpose

The primary purpose of the `zygisk-api` crate is to enable developers to write Zygisk modules entirely in Rust. This empowers them to leverage Rust's language features, such as memory safety, strong type system, and rich ecosystem, for Android system-level modifications.

### Problem Solved

Traditionally, Zygisk modules are written in C++. While powerful, C++ development can be prone to memory safety issues and often involves more verbose boilerplate for common tasks. Interfacing with Zygisk from other languages typically requires manual setup of Foreign Function Interface (FFI) bindings, which can be complex and error-prone.

The `zygisk-api` crate solves these problems by:
-   Providing pre-defined, safe Rust abstractions over the underlying Zygisk C API.
-   Handling the complexities of FFI, allowing developers to focus on module logic rather than inter-language communication details.
-   Offering a versioning system that adapts to different Zygisk API versions, reducing the burden on module developers to manage these changes themselves.
-   Simplifying common tasks like module and companion process registration through macros.

Ultimately, it allows Rust developers to easily hook into and modify Android's Zygote process (the progenitor of all app processes) and individual application processes without needing to directly write C++ or manually manage low-level FFI details.

### Main Components

The `zygisk-api` crate is structured around several key components:

1.  **`ZygiskModule` Trait:**
    *   **Role:** This trait is the primary interface for developers. To create a Zygisk module, a developer defines a struct and implements the `ZygiskModule` trait for it. This trait's methods correspond to different events in the Zygisk lifecycle.
    *   **Methods:**
        *   `on_load(api: ZygiskApi<Self::ApiVersion>, files_apk: Option<std::fs::File>)`: Called when the module is first loaded into a process (Zygote, app, or system server). This is the ideal place for one-time initializations and setup. The `api` object provides access to Zygisk functions, and `files_apk` (or an equivalent accessible via `api`) allows access to the module's own APK assets.
        *   `pre_app_specialize(api: ZygiskApi<Self::ApiVersion>, args: &mut <Self::ApiVersion as ZygiskRaw>::AppSpecializeArgs)`: Called in the Zygote process just *before* an application process is forked and specialized. It allows the module to modify app-specific properties (like UID, GID, SELinux context) by changing fields in the mutable `args` struct.
        *   `post_app_specialize(api: ZygiskApi<Self::ApiVersion>, args: &<Self::ApiVersion as ZygiskRaw>::AppSpecializeArgs)`: Called in the newly forked application process *after* it has been specialized but *before* any of its own code begins execution. This is where most app-specific modifications (e.g., hooking, loading custom code) occur. The `args` struct provides information about the app.
        *   `pre_server_specialize(api: ZygiskApi<Self::ApiVersion>, args: &mut <Self::ApiVersion as ZygiskRaw>::ServerSpecializeArgs)`: Similar to `pre_app_specialize`, but called in Zygote before the System Server process is specialized.
        *   `post_server_specialize(api: ZygiskApi<Self::ApiVersion>, args: &<Self::ApiVersion as ZygiskRaw>::ServerSpecializeArgs)`: Similar to `post_app_specialize`, but called in the System Server process after its specialization.

2.  **`ZygiskApi<Version>` Struct:**
    *   **Role:** This struct is a safe, high-level wrapper around the raw Zygisk C API functions. An instance of `ZygiskApi` is passed to the `ZygiskModule` trait methods, providing the module with a convenient way to interact with Zygisk.
    *   **Functionality:** It abstracts away the `unsafe` raw pointer manipulations and FFI calls. It provides methods for logging, accessing module files, getting process information, connecting to a companion process, and invoking Zygisk-specific functionalities like JNI or PLT hooking.
    *   **API Versioning:** It is generic over a `Version` type parameter (which must implement `ZygiskRaw`). This allows `ZygiskApi` to adapt its calls and expected data types to the specific Zygisk API version targeted by the module.

3.  **`ZygiskRaw<'a>` Trait:**
    *   **Role:** This trait is the cornerstone of the crate's API versioning mechanism. Each supported Zygisk API version (e.g., v1, v2, v5) has a corresponding "marker" struct (e.g., `raw::v1::V1`, `raw::v5::V5`) that implements `ZygiskRaw`.
    *   **Functionality:** It uses associated types (`ApiTable`, `AppSpecializeArgs`, `ServerSpecializeArgs`, `ModuleAbi`) to define the precise C-compatible structures and function table layouts for that specific Zygisk API version. It also includes an `API_VERSION` constant and methods like `abi_from_module` that handle the version-specific logic for constructing the C ABI representation of the module's callbacks. This allows the generic parts of the crate (`ZygiskApi`, `register_module!`) to work correctly with any supported Zygisk version.

4.  **`register_module!(MyModuleType, MyZygiskVersion)` Macro:**
    *   **Role:** This macro significantly simplifies the boilerplate required to expose a Rust struct (which implements `ZygiskModule<MyZygiskVersion>`) as a valid Zygisk module.
    *   **Functionality:** It generates the `extern "C"` entry point function (named `zygisk_module_entry` by convention) that Zygisk looks for when loading a module. This generated function handles the instantiation of the user's module struct, sets up the internal `RawModule` wrapper, prepares the version-specific `ModuleAbi` (which contains function pointers to the Rust callbacks), and calls the appropriate Zygisk function to register the module with the Zygisk framework.

5.  **`register_companion!(my_companion_fn)` Macro:**
    *   **Role:** This macro is used to set up an optional companion process for the module.
    *   **Functionality:** It defines another `extern "C"` entry point function (`zygisk_companion_entry` by convention). Zygisk launches this function in a separate process (often with different privileges, like root) if the module requests a companion. This is useful for tasks requiring elevated privileges or for operations that should not block Zygote or app processes.

### Interaction Between `api` and `raw` Modules

The crate typically has a two-tiered structure for managing Zygisk API interactions: a `raw` module and an `api` module.

*   **`raw` Module (e.g., `crate::raw`):**
    *   This module contains the direct FFI (Foreign Function Interface) bindings to the Zygisk C API. It includes:
        *   Definitions of C-compatible structs (e.g., `AppSpecializeArgsVx`, `ApiTableVx`) that mirror the memory layout of structures defined by the Zygisk C headers for different API versions (where `Vx` denotes a specific version like `V1`, `V5`). These are marked with `#[repr(C)]`.
        *   Type aliases for function pointers corresponding to the Zygisk API functions.
        *   Implementations of the `ZygiskRaw` trait for each supported API version.
    *   This layer is inherently `unsafe` as it deals with raw pointers and C data types. It is version-specific, with submodules like `raw::v1`, `raw::v2`, etc., each defining the specifics for that Zygisk API version.

*   **`api` Module (e.g., `crate::api`):**
    *   This module provides safe abstractions over the `raw` module. The central piece is the `ZygiskApi<Version>` struct.
    *   When a module developer implements `ZygiskModule<MyVersion>` and uses the `ZygiskApi<MyVersion>` instance passed to their callbacks, they are interacting with this safe layer.
    *   **Data Flow:** Calls to methods on `ZygiskApi<Version>` (e.g., `api.get_module_dir()`) are internally dispatched to the appropriate function pointers within the `Version::ApiTable` (the raw C function table for the specific Zygisk `Version`). For example, if `Version` is `raw::v5::V5`, `api.get_module_dir()` would eventually call `raw_api_table_v5->get_module_dir_fn(...)`.
    *   The `api` module ensures that data is correctly marshaled between Rust types and the C types expected by the `raw` functions.

*   **Ownership and Safety:**
    *   The `ZygiskApi` struct and associated helper functions aim to provide memory safety over the raw pointers and data types used by Zygisk. For instance, when Zygisk provides a C string (`*const c_char`), `ZygiskApi` might provide a safe way to convert it to a Rust `String`.
    *   It also manages the lifetimes of objects passed from Zygisk (often tied to the lifetime of the `JNIEnv` or the callback duration) to prevent use-after-free errors, primarily through the use of lifetime parameters (e.g., `'a` in `ZygiskApi<'a, Version>`).
    *   The overall design allows module developers to write most of their code in safe Rust, with the `unsafe` FFI interactions being encapsulated and managed by the crate itself.
