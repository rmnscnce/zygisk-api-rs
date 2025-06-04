## JNI Integration

The Java Native Interface (JNI) is a critical component for Zygisk modules, as Zygote, Android applications, and the Android System Server are all primarily Java-based environments. Even though `zygisk-api` allows modules to be written in Rust, interaction with the Java world is often necessary to achieve the module's goals.

### Fundamentals of JNI

*   **What is JNI?**
    The Java Native Interface is a foreign function interface programming framework that enables Java code running in a Java Virtual Machine (JVM) to call, and be called by, native applications and libraries written in other languages such as C, C++, and consequently, Rust.

*   **Why is JNI necessary for Zygisk modules?**
    1.  **Interacting with Android Frameworks:** Most Android APIs and system services are exposed as Java classes and methods. To modify application behavior, access system information, or interact with UI elements, Zygisk modules frequently need to call these Java APIs.
    2.  **Accessing Application Objects:** Modules often need to get information about the current application, such as its package name, `Context` object, `ClassLoader`, or specific Java classes and objects within the app.
    3.  **Calling Module Code from Java:** In some scenarios, a module might inject Java code that needs to call back into the Rust native code.
    4.  **Hooking Native Methods:** Java classes can declare `native` methods, whose implementations are provided by native libraries. JNI is the bridge that connects these Java `native` declarations to their actual C/C++/Rust implementations. Zygisk modules might want to hook these implementations or provide their own.

### How `zygisk-api` Facilitates JNI Usage

The `zygisk-api` crate aims to make JNI interactions safer and more convenient from Rust.

*   **The `jni` Crate:**
    `zygisk-api` typically builds upon and re-exports the `jni` crate. The `jni` crate provides relatively safe Rust bindings to the raw JNI C API, offering a more idiomatic Rust interface, handling many common pitfalls, and providing useful utility types. Developers using `zygisk-api` for JNI operations will effectively be using the features of the `jni` crate.

*   **`JNIEnv<'a>`:**
    *   **Primary JNI Interface:** The `JNIEnv` (Java Native Interface Environment) is the most crucial structure for interacting with the JVM from native code. It's a thread-local pointer that the JVM passes to each native method. It contains function pointers for all JNI functions (e.g., `FindClass`, `GetMethodID`, `CallObjectMethod`).
    *   **Accessibility:** In `zygisk-api`, the `JNIEnv` for the current thread is typically made available via a method on the `ZygiskApi` object, such as `api.get_jni_env()`. This `JNIEnv` is valid for the duration of the callback in which it was obtained.
    *   **Thread-Locality:** A `JNIEnv` pointer is only valid in the thread in which it was obtained. It cannot be shared across threads. If a module creates its own native threads that need to interact with Java, each thread must first attach to the JVM to get its own `JNIEnv`.

*   **`JavaVM`:**
    *   **JVM Representation:** The `JavaVM` struct represents the Java Virtual Machine instance. There is typically one `JavaVM` per process.
    *   **Functionality:** It allows native code to perform JVM-level operations, such as attaching and detaching native threads to the JVM (to obtain a `JNIEnv` for that thread) and getting a `JNIEnv` for the current thread.
    *   **Accessibility:** `zygisk-api` might provide access to the `JavaVM` instance, for example, via `api.get_java_vm()`. This is less commonly used directly in callbacks than `JNIEnv` but is essential for advanced scenarios like managing custom native threads.

### Common JNI Operations Relevant to Zygisk Modules

The following examples assume `env` is a `jni::JNIEnv` instance obtained via `api.get_jni_env()`.

*   **Finding Java Classes:**
    `let class_name = "java/lang/String";` // JNI format: package/name/ClassName
    `let j_class_string: JClass = env.find_class(class_name)?;`

*   **Getting Method and Field IDs:**
    `let method_id = env.get_method_id(j_class_string, "length", "()I")?;` // "()I" is JNI signature for `int length()`
    `let static_field_id = env.get_static_field_id(j_some_class, "SOME_CONSTANT", "Ljava/lang/String;")?;` // `L<path>;` for objects

*   **Calling Java Methods:**
    `let j_string_obj: JObject = ... ;`
    `let length_jvalue = env.call_method(j_string_obj, method_id, &[])?;`
    `let length_int: i32 = length_jvalue.i()?;` // Extract primitive from JValue
    `let static_string_obj: JObject = env.call_static_object_method(j_some_class, static_method_id, &[])?;`

*   **Accessing and Modifying Java Fields:**
    `let field_val_jvalue = env.get_field(j_object, field_id)?;`
    `env.set_field(j_object, field_id, JValue::from(123_i32))?;`

*   **Creating Java Objects:**
    `let new_jstring: JString = env.new_string("Hello from Rust")?;`
    `let new_obj: JObject = env.new_object(j_class, constructor_method_id, &[arg1.into(), arg2.into()])?;`

*   **Converting Between Rust Strings and Java Strings:**
    - Rust `&str` to Java `JString`: `let java_string: JString = env.new_string("Rust string")?;`
    - Java `JString` to Rust `String`:
      `let jni_str: JNIStr = env.get_string(&java_string)?;`
      `let rust_string: String = jni_str.into();` (or `String::from(jni_str)`)

*   **Handling Java Exceptions:** (See "Error Handling" below)

### Data Type Conversions

*   **Primitive Types:** Rust primitive types are mapped directly to JNI's primitive types (which correspond to Java's).
    *   `bool` <-> `jboolean` (typically `u8`)
    *   `i8` <-> `jbyte`
    *   `u16` <-> `jchar` (Java char is UTF-16)
    *   `i16` <-> `jshort`
    *   `i32` <-> `jint`
    *   `i64` <-> `jlong`
    *   `f32` <-> `jfloat`
    *   `f64` <-> `jdouble`
*   **Object Types (References):**
    *   All Java objects are represented in JNI by the opaque reference type `jobject`.
    *   More specific JNI reference types inherit from `jobject`, such as `jclass` (for `java.lang.Class` instances), `jstring` (for `java.lang.String`), `jarray` (and its typed variants like `jintArray`, `jobjectArray`), and `jthrowable` (for exceptions).
    *   The `jni` crate provides safe Rust wrapper types for these JNI references, like `JObject<'a>`, `JClass<'a>`, `JString<'a>`, `JThrowable<'a>`, which are tied to the lifetime `'a` of the `JNIEnv<'a>`. These wrappers automatically manage the deletion of local JNI references when they go out of scope.

### Memory Management in JNI

Proper memory management is crucial in JNI to prevent memory leaks and crashes. The Java Garbage Collector (GC) needs to be aware of any Java objects referenced by native code.

*   **Local References:**
    *   Most JNI functions that return object references (e.g., `FindClass`, `NewObject`, `GetObjectField`) create **local references**.
    *   **Lifecycle:** A local reference is valid only within the thread it was created in and during the scope of the single native method call that created it (i.e., typically for the duration of one `ZygiskModule` callback).
    *   **Automatic Release:** When a native method returns to Java, all local references created during its execution are automatically released by the JVM (unless explicitly managed otherwise). The `jni` crate's safe wrappers (`JObject`, `JString`, etc.) handle calling `DeleteLocalRef` when they are dropped.
    *   **Limits:** There's a limit to the number of local references a native thread can simultaneously hold (e.g., 16 or 512, depending on JVM/Android version). In loops creating many Java objects, you might need to explicitly delete local references using `env.delete_local_ref(obj_ref)` or use `env.with_local_frame { ... }` to manage their scope.

*   **Global References:**
    *   If a Java object needs to be held by native code across multiple native method calls, across different threads, or for an extended period (e.g., caching a `jclass` object), a **global reference** must be created from a local reference using `env.new_global_ref(local_obj_ref)?`.
    *   **Lifecycle:** Global references are not automatically garbage collected. They keep the Java object alive even if no Java code references it, until the global reference is explicitly deleted by native code.
    *   **Manual Deletion:** It is **critical** to delete global references using `env.delete_global_ref(global_ref_obj)?` when they are no longer needed by the native code. Failure to do so will result in memory leaks, as the Java object will never be garbage collected.

### Error Handling

JNI operations can result in Java exceptions. Native code must explicitly check for and handle these.

*   **Checking for Exceptions:** After a JNI call that might throw an exception, use `env.exception_check()?` (which returns a `Result<bool, jni::errors::Error>`). If it returns `Ok(true)`, an exception has occurred.
*   **Getting the Exception:** `let throwable: JThrowable = env.exception_occurred()?;` retrieves the pending exception object.
*   **Clearing Exceptions:** `env.exception_clear()?;` clears the pending exception. If not cleared, subsequent JNI calls may behave unexpectedly or crash.
*   **Propagating Exceptions:** If a native method was called from Java, it can choose to return immediately after an exception occurs (without clearing it), thereby propagating the exception to the Java caller.

```rust
// Example: Calling a Java method and checking for exceptions
match env.call_method(obj, mid, &[]) {
    Ok(result) => {
        // Process result
    }
    Err(jni::errors::Error::JavaException) => { // Specific error variant for Java exceptions
        let exc = env.exception_occurred()?;
        env.exception_clear()?; // Clear it
        // Handle the exception, e.g., log it, return a Rust error
        api.log_e(format!("A Java exception occurred: {:?}", exc));
    }
    Err(other_jni_error) => {
        // Handle other JNI errors
        api.log_e(format!("A JNI error occurred: {:?}", other_jni_error));
    }
}
```

### `hook_jni_native_methods` (Example from `ZygiskApi` v5)

Some Zygisk API versions (or this crate's wrappers for them) provide specialized functions for common tasks. For instance, `hook_jni_native_methods` found in `zygisk_api::api::v5::V5` (and other versions) allows replacing the implementations of `native` Java methods.

*   **Purpose:** To intercept calls to Java `native` methods and redirect them to custom Rust functions. This is a powerful way to modify or inspect data passed between Java and existing native libraries.
*   **Usage (Conceptual):** It typically involves providing the `JNIEnv`, the fully qualified class name (e.g., `"com/example/MyClass"`), and a slice of `JNINativeMethod` structs. Each `JNINativeMethod` struct would contain the method name (as a C string), its JNI signature (as a C string), and a function pointer (`fnPtr`) to the Rust replacement function. The original function pointers might be returned or stored in the provided slice.

### Potential Pitfalls

*   **Forgetting Exception Checks:** Not checking for exceptions after JNI calls can lead to crashes or undefined behavior when subsequent JNI calls are made with a pending exception.
*   **Mishandling References:**
    *   Using a local reference after the native method returns or in a different thread.
    *   Forgetting to delete global references (leading to memory leaks).
    *   Creating too many local references in a loop without deleting them, potentially exceeding JVM limits.
*   **Threading Issues:** `JNIEnv` is thread-local. Native threads created by the module must be attached to the JVM using `JavaVM::attach_current_thread()` to obtain a valid `JNIEnv` and detached using `JavaVM::detach_current_thread()` before they exit.
*   **Incorrect Method Signatures:** Providing an incorrect JNI method signature string when getting a method ID will result in a `NoSuchMethodError` being thrown (or an equivalent error).
*   **Class Loader Issues:** In Android, classes can be loaded by different class loaders. `env.find_class()` typically uses the class loader associated with the current Java call stack or the system class loader. If a class is loaded by a specific application class loader, you might need to obtain that `java.lang.ClassLoader` object first and then call its `loadClass` method via JNI to find the desired class.

A solid understanding of the `jni` crate's documentation, careful attention to reference lifetimes, and diligent error checking are essential for robust JNI integration in Zygisk modules.
