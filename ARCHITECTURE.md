# Overall Architecture

## Purpose

The `zygisk-api` crate provides comprehensive Rust bindings for the Zygisk API. Zygisk is a feature of Magisk that allows developers to run code in every Android application's process, as well as in the Zygote process itself. This crate empowers Rust developers to create Zygisk modules, enabling them to modify and extend the behavior of Android applications and the system at a fundamental level.

## Problem Solved

Traditionally, developing Zygisk modules required writing C++ code and manually managing Foreign Function Interface (FFI) details if other languages were involved. The `zygisk-api` crate solves this by:

-   **Enabling Rust Development:** Allowing developers to leverage Rust's safety, concurrency features, and rich ecosystem for Zygisk module development.
-   **Simplifying FFI:** Abstracting away the complexities of direct C/C++ interoperation. The crate handles the low-level details of calling Zygisk functions and accessing Zygisk data structures.
-   **Providing Type Safety:** Offering Rust types that correspond to Zygisk concepts, reducing the risk of errors associated with manual FFI.

In essence, it makes Zygisk module development more accessible and robust for the Rust community, allowing them to easily hook into and modify Android's Zygote process (the parent of all app processes) and individual application processes.

## Main Components

The `zygisk-api` crate is built around several key components that work together to provide a seamless development experience:

### 1. `ZygiskModule` Trait

This trait is the cornerstone for developers using this crate. It defines the interface that every Zygisk module must implement to respond to various events in the Zygisk lifecycle. It is generic over a type parameter `V: ZygiskRaw` which specifies the Zygisk API version the module targets.

-   **Role:** It serves as the primary entry point and behavior definition for a Rust-based Zygisk module. Developers implement the methods of this trait to execute their custom logic at specific stages.
-   **Methods (conceptual signatures, actual methods take `&self`):**
    -   `on_load(api: ZygiskApi<V>, _files_apk: Option<std::fs::File>)`: Called when the module is first loaded. `_files_apk` (or equivalent access via `api`) provides access to the module's own APK for assets.
        -   `api`: An instance of `ZygiskApi<V>` providing access to Zygisk functionalities for the specific API version `V`.
    -   `pre_app_specialize(api: ZygiskApi<V>, args: &mut V::AppSpecializeArgs)`: Called in the Zygote process *before* an application process is forked and specialized. Allows modification of app-specific properties via mutable `args`.
    -   `post_app_specialize(api: ZygiskApi<V>, args: &V::AppSpecializeArgs)`: Called in the newly forked application process *after* specialization but *before* its main code execution. `args` are immutable.
    -   `pre_server_specialize(api: ZygiskApi<V>, args: &mut V::ServerSpecializeArgs)`: Called in the Zygote process *before* the system server process is forked. Allows modification of server properties via mutable `args`.
    -   `post_server_specialize(api: ZygiskApi<V>, args: &V::ServerSpecializeArgs)`: Called in the system server process *after* specialization. `args` are immutable.

### 2. `ZygiskApi` Struct

This struct, `ZygiskApi<V: ZygiskRaw>`, acts as a safe and convenient high-level wrapper around the raw Zygisk functions.

-   **Role:** It provides an idiomatic Rust interface to interact with Zygisk. Instead of calling raw function pointers, developers use methods on a `ZygiskApi` instance.
-   **Functionality:** It abstracts details like API versioning (via the `V` type parameter) and unsafe pointer manipulation. It offers methods to:
    -   Log messages (e.g., `api.log_d(...)`).
    -   Access module files (e.g., `api.get_module_dir()`, `api.get_module_apk()`).
    -   Access the JNI environment (e.g., `api.get_jni_env()`).
    -   Connect to a companion process (e.g., `api.connect_companion()`).
    -   Utilize version-specific Zygisk features like PLT hooking or JNI hooking.
-   **API Versioning:** `ZygiskApi` is generic over a `ZygiskRaw` implementation `V`. This means it operates with a specific version of the underlying Zygisk C API (e.g., `ZygiskApi<zygisk_api::raw::v5::V5>`), determining which set of raw functions and types it uses.

### 3. `ZygiskRaw` Trait

This trait is fundamental to the crate's ability to support multiple Zygisk API versions.

-   **Role:** It defines an abstraction layer for version-specific Zygisk API details. Each Zygisk API version (e.g., v1, v2, v5) has a corresponding marker struct (e.g., `struct V5;`) that implements `ZygiskRaw`.
-   **Details:**
    -   It specifies associated types for version-specific data structures (like `ApiTable`, `AppSpecializeArgs`, `ServerSpecializeArgs`).
    -   It defines an `API_VERSION` constant.
    -   It provides functions like `abi_from_module` that handle version-specific logic for module registration.
-   **How `ZygiskApi` uses it:** `ZygiskApi<V: ZygiskRaw>` uses the types (e.g., `V::ApiTable`, `V::AppSpecializeArgs`) and function pointers defined by the `V` type. This allows `ZygiskApi`'s methods to remain consistent at the source level while the underlying calls are dispatched to the correct version-specific raw functions.

### 4. `register_module!` Macro

This macro, `register_module!(MyModuleType, MyZygiskVersion)`, significantly reduces the boilerplate code required to set up a Zygisk module.

-   **Role:** It generates the necessary FFI (Foreign Function Interface) glue code to expose a Rust struct (that implements `ZygiskModule<MyZygiskVersion>`) as a valid Zygisk module entry point.
-   **Functionality:**
    -   It creates the C-compatible entry point function (e.g., `zygisk_module_entry`) that Zygisk expects.
    -   It handles the instantiation of the `ZygiskModule` implementor.
    -   It manages the lifetime of the module instance.
    -   It ensures that the module's callback methods are correctly dispatched from C to Rust.

### 5. `register_companion!` Macro

This macro, `register_companion!(my_companion_entry_fn)`, is used to register an entry point for a companion process for the Zygisk module.

-   **Role:** Some modules may need a dedicated process running with different privileges (e.g., root) or for offloading tasks. This macro facilitates setting up such a companion service.
-   **Functionality:** It declares a Rust function as the entry point that Zygisk will execute in a new process. This function typically receives a file descriptor for communication with the main module.

## Interaction between `api` and `raw` Modules

The interaction between the `api` module (containing `ZygiskApi`) and the `raw` module (containing FFI bindings and `ZygiskRaw` implementations) is key to the crate's design, enabling both safety and version adaptability.

1.  **`raw` Module (The Unsafe Foundation):**
    *   Contains direct, low-level FFI bindings to the C/C++ Zygisk API. This includes definitions for C-structs, function pointer types (`ApiTable`), and `extern "C"` function declarations.
    *   For each supported Zygisk API version (e.g., `v1`, `v2`, `v5`), there's a sub-module (e.g., `zygisk_api::raw::vX`) defining:
        *   The raw Zygisk function table struct (e.g., `zygisk_api_table_vX`).
        *   Concrete types mirroring C types for that API version (e.g., `AppSpecializeArgsVx`).
        *   An implementation of the `ZygiskRaw` trait (e.g., `struct VX; impl ZygiskRaw for VX { ... }`).
    *   **Emphasis:** Code in the `raw` module is generally `unsafe` to interact with directly.

2.  **`api` Module (The Safe Abstraction):**
    *   Provides the `ZygiskApi<V: ZygiskRaw>` struct, offering a safe, high-level interface.
    *   When `ZygiskApi` is instantiated (e.g., `ZygiskApi::<zygisk_api::raw::v5::V5>::new(raw_api_ptr)`), it is parameterized by a specific `ZygiskRaw` implementation `V`.
    *   It holds an internal pointer/reference to the raw Zygisk API function table (`V::ApiTable`).

3.  **Data Flow and Dispatch:**
    *   Calls from module logic (via `ZygiskApi<V>`) are dispatched to the correct raw functions based on the chosen API version `V`.
    *   `ZygiskApi<V>` uses the function pointers from `V::ApiTable` and the associated types from `V` (like `V::AppSpecializeArgs`) to interact with the Zygisk framework.

4.  **Ownership and Safety:**
    *   `ZygiskApi` aims to provide memory safety over raw pointers and types. For instance, it might convert C strings to Rust `String`s or manage JNI local references.
    *   It encapsulates `unsafe` calls to the `raw` layer, presenting a safer interface.

This structure allows developers to write code against a stable Rust API (`ZygiskApi` and `ZygiskModule`) while the crate manages version-specific complexities.

# Memory Management, Lifetimes, and C ABI Compatibility

Interfacing Rust code with a C-based system like Zygisk requires careful attention to memory management, data lifetimes, and the C Application Binary Interface (ABI). These considerations are crucial for preventing crashes, memory leaks, and undefined behavior.

## Memory Management and Lifetimes

Rust's strong compile-time memory safety guarantees, primarily through its ownership and borrowing system, must be carefully managed at the FFI boundary where Rust code interacts with C code from Zygisk.

*   **Rust's Ownership and Borrowing:** At its core, Rust ensures memory safety without a garbage collector by enforcing strict rules about data ownership and borrowing. Each value in Rust has a variable that's its owner. There can only be one owner at a time. When the owner goes out of scope, the value will be dropped. This system prevents common issues like dangling pointers and data races in safe Rust code. When calling C code or being called from C, Rust's `unsafe` keyword is often involved, requiring developers to manually uphold these safety invariants for raw pointers and external data.

*   **`Box::leak` for Module Structures:**
    *   When a Zygisk module is initialized (typically within the `zygisk_module_entry` function generated by the `register_module!` macro), the crate creates heap-allocated instances of internal structures. These include a wrapper around the user's `ZygiskModule` implementation (often called `RawModule`) and the `ModuleAbi` struct which contains function pointers to the module's Rust callbacks.
    *   These Rust-managed structures must remain valid for the entire lifetime of the Zygisk module as perceived by the Zygisk framework. Zygisk will hold raw pointers to `ModuleAbi` and will invoke these function pointers at various lifecycle events.
    *   To achieve this persistence and transfer effective ownership to the C side, the crate uses `Box::leak()`. This function consumes a `Box<T>` (which represents heap-allocated data), forsakes Rust's automatic memory management for this allocation, and returns a raw pointer `*mut T` that has a `'static` lifetime. This means Rust considers the pointer valid for the entire program duration from its perspective.
    *   **Why `Box::leak`?** It's a way to tell Rust, "This memory will be managed externally now." Zygisk, being a C framework, isn't aware of Rust's ownership system or `Box<T>`. It only understands raw pointers. Leaking the box ensures the underlying data isn't deallocated by Rust when the original `Box` might go out of scope in the initialization function, thus keeping the pointer valid for Zygisk.
    *   **Implications:**
        1.  **No Automatic Deallocation by Rust:** Rust will not automatically call `drop` on the leaked data, nor will it deallocate the memory.
        2.  **OS Reclaims Memory:** This is generally acceptable because the Zygisk module's lifecycle is tied to the host process (Zygote, app, or system server). When this process terminates, the operating system reclaims all its memory, including any "leaked" allocations from the Rust module.
        3.  **Limited Scope:** The number of such core module structures that are leaked is typically very small (e.g., one `RawModule` instance per loaded module), minimizing the impact of this explicit memory leak.

*   **Lifetimes (`'a`) in Key Structs and Callbacks:**
    *   Lifetime parameters (e.g., `'a`) are pervasive in structs that handle data passed from Zygisk or JNI, such as:
        *   `ZygiskApi<'a, Version>`: Its lifetime is often tied to the Zygisk API table pointer.
        *   References to Zygisk-provided data, like `&'a Version::ApiTable`.
        *   Argument structs for callbacks, e.g., `AppSpecializeArgs<'a>`, `ServerSpecializeArgs<'a>`. These often contain references to Java objects (via `JNIEnv<'a>`) or data buffers provided by Zygisk.
    *   **Purpose:** Lifetimes ensure **reference safety**. They guarantee that references do not outlive the data they point to. In the FFI context:
        *   When Zygisk calls a Rust callback (e.g., `post_app_specialize`), it provides pointers to data (like `AppSpecializeArgs`). This data is typically valid only for the duration of that specific callback.
        *   The lifetime `'a` is used to scope references to this transient data. For example, `JString<'a>` in `AppSpecializeArgs<'a>` wraps a JNI `jstring` that is only valid as long as the current `JNIEnv<'a>` is valid (i.e., during the callback).
        *   By tying the lifetimes of Rust wrapper types to `'a`, the Rust compiler can prevent these wrappers (and any references derived from them) from being stored or used beyond the callback, thus preventing use-after-free bugs.

*   **Memory for JNI Objects:**
    *   As detailed in the "JNI Integration" section, Java objects passed to native callbacks (e.g., `jstring` in `AppSpecializeArgs`) are typically JNI **local references**.
    *   `JNIEnv` manages these. They are valid only for the duration of the native method call unless explicitly converted to JNI global references. The `jni` crate's wrapper types like `JString<'a>` are bound by the lifetime `'a` of the `JNIEnv<'a>` to reflect this transience.

## C ABI Compatibility

For seamless interaction between Rust and Zygisk (a C/C++ framework), Rust code must adhere to the C Application Binary Interface (ABI). This governs how data is laid out in memory and how functions are called.

*   **`#[repr(C)]`:**
    *   Rust's default memory layout for structs is optimized for space and may involve reordering fields. This layout is not stable and not predictable by C code.
    *   The `#[repr(C)]` attribute must be applied to any Rust struct that will be shared with C code or whose memory layout needs to match a C struct definition. This includes:
        *   The `ModuleAbi` struct (passed to Zygisk, containing function pointers).
        *   Version-specific raw API table structs (e.g., `raw::v1::ApiTable`).
        *   Argument structs like `AppSpecializeArgs` and `ServerSpecializeArgs` (passed by pointer from Zygisk to module callbacks).
    *   `#[repr(C)]` ensures that Rust lays out the struct fields in the order they are declared, matching C's behavior.

*   **Function Pointers and Calling Conventions (`extern "C" fn(...)`):**
    *   When C code calls a Rust function, or Rust code calls a C function (or a function pointer expected by C), they must agree on the **calling convention**. This defines how function arguments are passed (e.g., in registers or on the stack), how results are returned, and who is responsible for cleaning up the stack.
    *   The `extern "C"` keyword specifies that a function or function pointer should use the C calling convention.
    *   This is critical for:
        *   The main module entry point (e.g., `zygisk_module_entry`) generated by `register_module!`.
        *   All function pointers stored in `ModuleAbi` that Zygisk will call (e.g., `pre_app_specialize_fn`).
        *   Any function pointers obtained from Zygisk's `ApiTable` that Rust code will call.

*   **`NonNull<T>`:**
    *   In C, pointers can be `NULL`. In Rust, references are always non-null, but raw pointers (`*const T`, `*mut T`) can be null.
    *   `std::ptr::NonNull<T>` is a wrapper around a raw pointer `*mut T` that asserts the pointer is never null. This is useful for FFI:
        *   If the C API guarantees a pointer will not be null (e.g., a pointer to the module's own instance or a valid API table), `NonNull<T>` can express this invariant in Rust.
        *   It allows for some compiler optimizations similar to those for references and can make code more self-documenting about nullability expectations.
        *   Dereferencing `NonNull<T>` still requires an `unsafe` block.

*   **Primitive Types:**
    *   For FFI, Rust uses C-compatible primitive types, often from the `libc` crate or `std::os::raw` (which re-exports `libc` types). Examples:
        *   `libc::c_int`, `libc::c_uint`
        *   `libc::c_long`, `libc::c_ulong`
        *   `libc::c_char` (for C strings)
        *   `libc::c_void` (Rust's `()` is often used for C's `void` return, but `*mut libc::c_void` is Rust's `*mut ()`)
    *   These are used in struct definitions (e.g., `ApiTable` fields, `AppSpecializeArgs` fields that are not JNI types) and function signatures to match the types defined by the Zygisk C API.

By meticulously managing these FFI aspects, `zygisk-api` aims to create a safe and reliable bridge between Rust's memory-safe environment and the C-based Zygisk framework.

# Type System and API Design

The `zygisk-api` crate leverages several of Rust's type system features to provide a safe, flexible, and extensible interface for Zygisk module development. This "type system wizardry" is key to how the crate shapes a user-friendly API over the raw Zygisk C functions.

## Leveraging Traits for Extensibility and Versioning

Traits are Rust's primary mechanism for defining shared behavior (interfaces) and are central to the crate's design.

*   **`ZygiskModule<Version>` Trait:**
    *   This trait is the main way users define their module. Its methods (`on_load`, `pre_app_specialize`, etc.) provide clear, overridable hooks into Zygisk lifecycle events.
    *   The `Version` type parameter (constrained by `ZygiskRaw`) is crucial:
        *   It allows module logic to be generic over Zygisk API versions.
        *   The *specific* `Version` type chosen by the developer (e.g., `raw::v5::V5`) determines the concrete types of arguments (like `V::AppSpecializeArgs`) passed to the trait methods, ensuring version-correctness.

*   **`ZygiskRaw<'a>` Trait:**
    *   This trait is the cornerstone of the crate's API versioning. Each supported Zygisk API version has a unique marker type (e.g., `raw::v1::V1`, `raw::v5::V5`) that implements `ZygiskRaw<'a>`. These implementations act as the concrete "version definitions."
    *   **Associated Types:** The power of `ZygiskRaw` lies in its associated types:
        *   `type ApiTable`: Defines the exact structure of the raw Zygisk function table for that version.
        *   `type AppSpecializeArgs`, `type ServerSpecializeArgs`: Define the precise layout of argument structs Zygisk passes to callbacks for that version.
        *   `type ModuleAbi`: Defines the C-compatible struct holding function pointers to the module's Rust callbacks, tailored for that version.
        By using these associated types, generic code (like `ZygiskApi` and `ZygiskModule` trait methods) can refer to version-specific structures in a type-safe way.
    *   **Version-Specific Logic:** Methods within `ZygiskRaw` (e.g., `abi_from_module`, `register_module_fn`) encapsulate logic that differs between Zygisk versions, such as how the `ModuleAbi` is constructed or how the module is registered with the Zygisk framework. This abstracts these low-level, version-dependent details from the user.

## Generics and Type Safety in `ZygiskApi`

The `ZygiskApi` struct uses generics to provide a type-safe and version-aware interface to Zygisk functions.

*   **`ZygiskApi<'a, Version: ZygiskRaw<'a>>`:**
    *   This struct is the safe, high-level entry point for modules to interact with Zygisk (e.g., `api.connect_companion()`).
    *   It's generic over a lifetime `'a` (often tied to `JNIEnv` or Zygisk-provided data) and a `Version` type that must implement `ZygiskRaw<'a>`.
    *   **Type-Safe Dispatch:** Methods on `ZygiskApi` call function pointers from the `Version::ApiTable`. Because `Version` dictates the specific `ApiTable` type, `ZygiskApi` automatically uses the correct set of function pointers for the targeted Zygisk API version.
    *   **Compile-Time Guarantees:** The `Version: ZygiskRaw<'a>` bound ensures `ZygiskApi` can only be used with a valid, fully defined API version. This means that if a module developer specifies they are using `ZygiskApi<raw::v5::V5>`, the compiler ensures all calls made through `api` use v5 function signatures and that data structures (like arguments to callbacks) conform to v5 definitions. This prevents many potential runtime errors due to API version mismatches.

## Macros for Boilerplate Reduction and Entry Point Definition

Macros are a powerful metaprogramming feature in Rust, used here to reduce boilerplate and handle FFI complexities.

*   **`register_module!($module_type, $version_type)` Macro:**
    *   Simplifies module creation by automatically generating the `extern "C" fn zygisk_module_entry(...)` C-ABI compatible function. This is the actual entry point Zygisk calls.
    *   It takes the user's module struct type (implementing `ZygiskModule<$version_type>`) and the Zygisk version marker type.
    *   The macro expands to code that correctly instantiates the user's module, sets up internal wrappers (`RawModule`), prepares the version-specific `ModuleAbi` (using `$version_type::abi_from_module`), and calls Zygisk's registration function.
    *   **Panic Safety:** Crucially, the generated `zygisk_module_entry` often wraps calls to the user's module code (like `on_load` or specialization callbacks) in `std::panic::catch_unwind`. This prevents Rust panics from unwinding across the FFI boundary into C code, which is undefined behavior and would likely crash the process. Instead, it allows for a more controlled shutdown or error logging.

*   **`register_companion!($func)` Macro:**
    *   Similarly generates the `extern "C" fn zygisk_companion_entry(...)` for the companion process, abstracting FFI details from the user who only provides their Rust function name.

## The Role of `PhantomData`

`std::marker::PhantomData<T>` is a zero-sized "marker" type. It's used when a struct definition needs to signal to the Rust compiler that it logically "acts as if" it contains a field of type `T`, even if `T` isn't actually stored. This is important for Rust's type system rules, especially with lifetimes and generic parameters in `unsafe` code or FFI.

*   **Example Usage & Purpose:**
    *   A struct like `ZygiskApi<'a, Version>` might internally use `PhantomData<&'a ()>` or `PhantomData<Version>`.
    *   **Lifetime Binding with Raw Pointers:** If a struct holds raw pointers (which don't have lifetimes in Rust's type system like references do) but logically depends on some lifetime `'a` (e.g., the lifetime of a `JNIEnv<'a>` or data passed into a callback), `PhantomData<&'a ()>` can be used. It tells the Rust compiler that instances of this struct are tied to `'a`, ensuring the struct isn't used beyond where `'a` is valid. This helps the borrow checker maintain safety even when raw pointers are involved.
    *   **Marking Generic Parameters as Used:** If a struct is generic over a type `Version` but `Version` isn't used in any of its fields (perhaps because `Version` only dictates types used in method signatures or associated types), `PhantomData<Version>` signals that `Version` is logically part of the struct's definition. This prevents compiler errors about unused generic parameters and correctly informs variance calculations.
    *   In essence, `PhantomData` helps ensure that Rust's compile-time checks (lifetimes, variance, drop check) are correctly applied, even for types that have complex interactions with `unsafe` code or FFI.

By employing these advanced Rust type system features, `zygisk-api` provides an interface that is not only functional and ergonomic but also significantly safer and more robust than manual FFI management.

# API Versioning
[...]
# Overall Architecture
[...]
# Module Lifecycle and Callbacks
[...]
# JNI Integration
[...]
# Companion Process
[...]
# API Version Overview
[...]
