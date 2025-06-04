## API Versioning

The Zygisk API, like any evolving software interface, introduces changes, new features, and deprecations over time. To manage this, Zygisk employs an API versioning system. Zygisk modules must be aware of these versions to ensure they function correctly with the specific version of Zygisk running on a user's device. The `zygisk-api` crate is designed to make handling these API versions as straightforward as possible for Rust developers.

### The Need for API Versioning in Zygisk

1.  **Evolving Features:** As Magisk and Zygisk develop, new functionalities are added to allow modules more capabilities or finer-grained control. For example, new ways to interact with the Zygote process, modify app specialization, or communicate with other parts of the system might be introduced.
2.  **Signature Changes:** Existing Zygisk functions might change their signatures (parameter types, return types) to accommodate new features, fix bugs, or improve ergonomics.
3.  **Structural Changes:** Data structures passed to and from Zygisk (e.g., arguments for app specialization) might have fields added, removed, or altered.
4.  **Compatibility:** A module compiled against a specific Zygisk API version (e.g., Zygisk v2) might not be compatible with a system running a different version (e.g., Zygisk v4). Using mismatched versions could lead to crashes, incorrect behavior, or an inability for the module to load.
5.  **Explicit Targeting:** Zygisk requires modules to declare which API version they are built for. This allows Zygisk to provide the correct function tables and data structures to the module and to determine if the module is compatible.

### How `zygisk-api` Supports Multiple API Versions

The `zygisk-api` crate addresses the challenge of API versioning through a combination of Rust's type system features, particularly traits and generics, along with a modular file structure.

1.  **Version-Specific Modules:**
    *   The crate organizes version-specific code into separate Rust modules. Typically, you'll find these within the `src/raw/` directory (e.g., `src/raw/v1.rs`, `src/raw/v2.rs`, `src/raw/v5.rs`) and potentially corresponding modules in `src/api/` if safe wrappers have version-specific aspects.
    *   **`src/raw/vX.rs`:** These modules contain the direct FFI (Foreign Function Interface) definitions for a specific Zygisk API version `X`. This includes:
        *   The C-compatible struct definition for the Zygisk API function table for that version (e.g., `ApiTableV1`, `ApiTableV5`). This table holds the function pointers provided by Zygisk.
        *   C-compatible struct definitions for arguments passed to module callbacks, specific to that version (e.g., `AppSpecializeArgsV1`, `ServerSpecializeArgsV1`).
        *   An implementation of the `ZygiskRaw` trait for a version marker type (e.g., `struct V1; impl ZygiskRaw for V1 { ... }`).
    *   **`src/api/vX.rs` (or generic `src/api.rs`):** This layer provides the safe `ZygiskApi<Version>` wrapper. While `ZygiskApi` itself is generic, its interaction with the `raw` layer is dictated by the specific `Version` type parameter, which corresponds to one of the `raw::vX` definitions.

### The Role of the `ZygiskRaw` Trait

The `ZygiskRaw<'a>` trait is the central abstraction that enables robust API versioning in this crate.

*   **Abstraction for Versioning:** It defines a contract that each specific Zygisk API version's marker type (e.g., `raw::v1::V1`, `raw::v5::V5`) must implement. This contract specifies all the elements that can differ between Zygisk versions.
*   **Associated Types:** The true power of `ZygiskRaw` for versioning comes from its associated types. These act as placeholders that each implementing version-specific type fills in with concrete definitions:
    *   `type ApiTable`: Defines the actual Rust type for the raw Zygisk API function table for that version (e.g., `raw::v1::ApiTable` for `V1`).
    *   `type AppSpecializeArgs`: Defines the Rust struct for the arguments passed to `pre_app_specialize` and `post_app_specialize` for that version.
    *   `type ServerSpecializeArgs`: Defines the Rust struct for the arguments passed to `pre_server_specialize` and `post_server_specialize` for that version.
    *   `type ModuleAbi`: Defines the C-compatible struct that holds function pointers to the module's Rust callbacks, tailored for that specific Zygisk version's expectations.
*   **`API_VERSION` Constant:** Each implementation of `ZygiskRaw` must define a `const API_VERSION: u32` (or `c_long`). This constant is used by the `register_module!` macro to inform Zygisk which API version the module is targeting.
*   **Version-Specific Functions:** `ZygiskRaw` can also define methods that require version-specific implementations. For example:
    *   `abi_from_module(...)`: This function is responsible for taking the crate's internal representation of a module and constructing the correct `ModuleAbi` struct (using the associated type `Self::ModuleAbi`) with the appropriate function pointers for that version.
    *   `register_module_fn(...)`: This method might return a function pointer to the specific Zygisk function responsible for module registration, as this too could vary.

By implementing `ZygiskRaw`, each version marker (e.g., `raw::v5::V5`) provides all the necessary type information and version-specific logic needed for the generic parts of the crate to operate correctly for that Zygisk API version.

### Conceptual Example of Versioned API Usage

The `ZygiskApi` struct and the `ZygiskModule` trait are generic over a `Version` parameter, which is constrained by `ZygiskRaw`.

```rust
// Simplified ZygiskApi struct
pub struct ZygiskApi<'a, Version: ZygiskRaw<'a>> {
    internal_api_table: &'a Version::ApiTable, // Uses the ApiTable associated type
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, Version: ZygiskRaw<'a>> ZygiskApi<'a, Version> {
    pub fn get_module_dir(&self) -> Option<std::path::PathBuf> {
        // Accesses a function pointer like self.internal_api_table.get_module_dir_fn
        // The exact function pointer comes from the concrete ApiTable type defined by 'Version'.
        // ...
        None // Placeholder
    }
}

// Developer defines their module, choosing a specific Zygisk API version
use zygisk_api::{api::ZygiskApi, ZygiskModule};
use zygisk_api::raw::v5::V5; // Target Zygisk API v5

pub struct MyModule;

impl ZygiskModule<V5> for MyModule { // Implement ZygiskModule for V5
    type ApiVersion = V5; // Specify the API version type

    fn on_load(&self, api: ZygiskApi<Self::ApiVersion>, _files: Option<std::fs::File>) {
        // Here, 'api' is ZygiskApi<V5>.
        // Calls like api.get_module_dir() will use functions from V5::ApiTable.
        api.log_d("My Zygisk v5 Module Loaded!");
    }

    fn post_app_specialize(&self, api: ZygiskApi<Self::ApiVersion>,
                          args: &<Self::ApiVersion as ZygiskRaw>::AppSpecializeArgs) {
        // 'args' is of type &raw::v5::transparent::AppSpecializeArgs
        // The structure of 'args' is specific to Zygisk API v5.
        if let Some(nice_name_jstr) = args.nice_name.as_ref() {
            // ...
        }
    }
    // ... other trait methods
}

// Register the module, specifying its type and the Zygisk API version it targets
zygisk_api::register_module!(MyModule, V5);
```

In this example:
-   The developer explicitly chooses `V5` (the marker type for Zygisk API v5 which implements `ZygiskRaw`).
-   When `MyModule` implements `ZygiskModule<V5>`, all methods receive a `ZygiskApi<V5>` instance and argument types (like `V5::AppSpecializeArgs`) that are specific to Zygisk API v5.
-   Rust's trait system and type checking ensure that all types are consistent and that the correct version-specific function tables and data structures are used throughout the module's code and its interaction with the `zygisk-api` crate.

### Handling Differences Across API Versions

The `ZygiskRaw` trait and its associated types are the primary way the crate handles differences:

-   **Different Function Signatures or Availability:** If a Zygisk function (e.g., `connect_companion`) has different parameters or is only available from a certain API version onwards, the `ApiTable` associated type for each `ZygiskRaw` implementation will reflect this.
    -   The `ZygiskApi` struct's methods will then call the version-specific function pointers from the appropriate `ApiTable`.
    -   If a function is not available in a particular version, its corresponding field might be absent in that version's `ApiTable`, or the `ZygiskApi` method might be conditionally compiled or return an error/`None` for that version.
-   **Different Data Structures:** The `AppSpecializeArgs` and `ServerSpecializeArgs` associated types ensure that the module callbacks always receive arguments with the correct fields and layout for the targeted Zygisk API version. For instance, if `AppSpecializeArgs` gains a new field in Zygisk v4, `raw::v4::V4::AppSpecializeArgs` will include this field, while `raw::v3::V3::AppSpecializeArgs` will not.

This design allows `zygisk-api` to provide a stable Rust interface to module developers while internally managing the complexities of adapting to various Zygisk C API versions. The `register_module!` macro further assists by using the `ZygiskRaw::API_VERSION` constant to correctly declare the module's targeted API level to the Zygisk framework.
