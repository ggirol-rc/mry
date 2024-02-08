# Mry

[![GitHub](https://img.shields.io/badge/GitHub-ryo33/mry-222222)](https://github.com/ryo33/mry)
![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)
[![Crates.io](https://img.shields.io/crates/v/mry)](https://crates.io/crates/mry)
[![docs.rs](https://img.shields.io/docsrs/mry)](https://docs.rs/mry)
![GitHub Repo stars](https://img.shields.io/github/stars/ryo33/mry?style=social)

A simple but powerful mocking library for **structs**, **traits**, and **function**.

## Features

* A really simple and easy API
* Supports mock of structs, traits, and functions.
* No need to switch between mock objects and real objects.
* Supports partial mocking.

## Compared to [mockall](https://github.com/asomers/mockall)

The clear difference between mry is that the API is simple and small, and since it is still being developed, you would find some behaviors that are not yet supported.
Also, based on the principle of least astonishment, mry solves several problems of mockall in the simplest way.

**Mry is cfg-free**

In mockall, `#[double]` is used to switch real and mocked structs.
The problem is that `#[double]` makes mocked structs to be used for all test cases, so it will be complicated when some test case needs the real structs, especially for testing the struct itself.

In mry, no `#[double]` or complex `use` strategy is required. 

**Mry doesn't cause data races**

In mockall, you need a manual synchronization with a mock of static functions and methods. The problem is that the result will be unpredictable and hard to debug when you forget to have a lock.

In mry, there is a managed synchronization, and when you forget it, you can get an error that tells you it is required.

## Example

```rust
#[mry::mry]
struct Cat {
    name: String,
}

#[mry::mry]
impl Cat {
    fn meow(&self, count: usize) -> String {
        format!("{}: {}", self.name, "meow".repeat(count))
    }
}

#[test]
fn meow_returns() {
    let mut cat = mry::new!(Cat { name: "Tama".into() });

    let mock_meow = cat.mock_meow(mry::Any).returns("Called".to_string());

    assert_eq!(cat.meow(2), "Called".to_string());

    mock_meow.assert_called(1);
}
```

## How to mock a method or function

### Step1. Creating a pattern for a method or function

a. If you have a mock object `cat`: you can create a pattern for a method called `meow` by calling `cat.mock_meow` with a matcher for each argument.

```rust
// If you mock a struct called `Cat`
let mut cat = mry::new!(Cat { name: "Tama".into() });
// If you mock a trait called `Cat`
let mut cat = MockCat::default();

cat.mock_meow(mry::Any) // Any value can be matched
cat.mock_meow(3) // matched with 3
```

b. If you mock an associated function called `new` in a struct `Cat`, you can create a pattern with `Cat::mock_new`.

```rust
Cat::mock_new(mry::Any)
```

c. If you mock a function called `hello`, you can create a pattern with `mock_hello`.

```rust
mock_hello(mry::Any)
```


> [!NOTE]
> You can create multiple patterns for the same method or function, and they are matched in the order they are created.

### Step 2. Setting an expected behavior for the pattern

Followed by the pattern, you can chain one of the following to set the expected behavior.

- `returns(value)` - Returns a value always. The value must implement `Clone` for returning it multiple times.
- `returns_once(value)` - Returns a value only once. No need to implement `Clone`.
- `returns_with(closure)` - Returns a dynamic value by a closure that takes the arguments. No need to implement `Clone` for the output.
- `returns_ready(value)` - Returns a boxed future every time with cloning the value.
- `returns_ready_once(value)` - Shortcut for `returns_once(Box::new(async { value }))`.
- `calls_real_impl()` - Calls the real implementation of the method or function. Used for partial mocking.

> [!NOTE]
> `returns_ready` and `returns_ready_once` is not for `async fn`, but for methods that return a boxed future or trait methods with return-position `impl Future`.

```rust
cat.mock_meow(3).returns("Called with 3".into());
Cat::mock_new(mry::Any).returns(cat);
mock_hello(mry::Any).returns("World".into());
```

### (Optional) Step3. Asserting the pattern is called as expected times

You can call `assert_called` for asserting the pattern is called as expected times.

```rust
// You need to bind the mock object to a variable to call `assert_called`.
let mock_meow = cat.mock_meow(3).returns("Returns this string when called with 3".into());

assert_eq!(cat.meow(3), "Returns this string when called with 3".to_string());

mock_meow.assert_called(1);
```

Also, you can count for a specific pattern without setting a behavior.

```rust
// You must create a pattern before `cat.meow` would be called.
let mock_meow_for_2 = cat.mock_meow(2);

let mock_meow_for_any = cat.mock_meow(mry::Any).returns("Called".into());

assert_eq!(cat.meow(1), "Called".to_string());
assert_eq!(cat.meow(2), "Called".to_string());
assert_eq!(cat.meow(3), "Called".to_string());

mock_meow_for_2.assert_called(1);
mock_meow_for_any.assert_called(3);
```

## Basic Usages

### Mocking a struct

We need to add an attribute `#[mry::mry]` in the front of the struct definition and the impl block to mock them.

```rust
#[mry::mry] // This
struct Cat {
    name: &'static str,
}

#[mry::mry] // And this
impl Cat {
    fn meow(&self, count: usize) -> String {
        format!("{}: {}", self.name, "meow".repeat(count))
    }
}
```

`#[mry::mry]` adds a visible but ghostly field `mry` to your struct, so your struct must be constructed by the following ways.

```rust
// An easy way
mry::new!(Cat { name: "Tama" })

// is equivalent to:
Cat {
    name: "Tama",
    mry: Default::default(),
};

// If you derive or impl Default trait.
Cat::default();
// or
Cat { name: "Tama", ..Default::default() };
```

> [!IMPORTANT]
> When release build, the `mry` field of your struct will be zero sized, and `mock_*` functions will be unavailable.

### Partial mocks

You can do partial mocking by using `calls_real_impl()`.

```rust
#[mry::mry]
impl Cat {
    fn meow(&self, count: usize) -> String {
        self.meow_single().repeat(count)
    }

    fn meow_single(&self) -> String {
        "meow".into()
    }
}

#[test]
fn partial_mock() {
    let mut cat: Cat = Cat {
        name: "Tama".into(),
        ..Default::default()
    };

    cat.mock_meow_single().returns("hello".to_string());

    cat.mock_meow(mry::Any).calls_real_impl();

    // not "meowmeow"
    assert_eq!(cat.meow(2), "hellohello".to_string());
}
```

### Mocking a trait

Just add `#[mry::mry]` to the trait definition.

```rust
#[mry::mry]
pub trait Cat {
    fn meow(&self, count: usize) -> String;
}
```

Now we can use `MockCat` as a mock object.

```rust
// You can construct it by Default trait
let mut cat = MockCat::default();

// API's are the same as the struct mocks.
cat.mock_meow(2).returns("Called with 2".into());

assert_eq!(cat.meow(2), "Called with 2".to_string());
```

### Mocking a function

Add `#[mry::mry]` to the function definition.

```rust
#[mry::mry]
fn hello(count: usize) -> String {
    "hello".repeat(count)
}
```

We need to acquire a lock of the function by using `#[mry::lock(hello)]` because mocking of the static function uses a global state.

```rust
#[test]
#[mry::lock(hello)] // This is required!
fn function_keeps_original_function() {
    // Usage is the same as the struct mocks.
    mock_hello(Any).calls_real_impl();

    assert_eq!(hello(3), "hellohellohello");
}
```

### Mocking an associated function (static function)

Include your associated function into the impl block with `#[mry::mry]`.

```rust
#[mry::mry]
struct Cat {}

#[mry::mry]
impl Cat {
    fn meow(count: usize) -> String {
        "meow".repeat(count)
    }
}
```

We need to acquire a lock for the same reason in the mocking function above.

```rust
#[test]
#[mry::lock(Cat::meow)] // This is required!
fn meow_returns() {
    // Usage is the same as the struct mocks.
    Cat::mock_meow(Any).returns("Called".to_string());

    assert_eq!(Cat::meow(2), "Called".to_string());
}
```

To lock multiple static functions simultaneously, list the functions in a comma-separated format: `#[mry::lock(function_a, function_b, function_c)]`. This approach automatically prevents deadlocks by sorting the functions before locking.

## Advanced Usages

### `async fn` in trait (1.75.0 or later)

Add `#[mry::mry]` to the trait definition.

```rust
#[mry::mry]
pub trait Cat {
    async fn meow(&self, count: usize) -> String;
}
```

You can do `cat.mock_meow().returns("Called".to_string())` as the same as sync methods.

### trait_variant::make with `async fn` (1.75.0 or later)

If you use `trait_variant::make` attribute, you must put `#[mry::mry]` under the `#[trait_variant::make(Cat: Send)]`.

```rust
#[trait_variant::make(Cat: Send)]
#[mry::mry]
pub trait LocalCat {
    async fn meow(&self) -> String;
}

let mut cat = MockLocalCat::default();
cat.mock_meow().returns("Called".to_string());

let mut cat = MockCat::default();
cat.mock_meow().returns_ready("Called".to_string());
```

> [!IMPORTANT]
> You must use `returns_ready` for the generated one instead of `returns` for the original one. 
> Because the generated one is not `async fn` but `impl Future` in return position, and mry expects a boxed future for returning as `impl Future`.

### async_trait

If you use `async_trait` crate, you must put `#[async_trait]` under the `#[mry::mry]`.

```rust
#[mry::mry]
#[async_trait::async_trait]
pub trait Cat {
    async fn meow(&self, count: usize) -> String;
}
```

You can do `cat.mock_meow().returns("Called".to_string())` as the same as sync methods.

### impl Trait for Struct

Mocking of impl trait is supported in the same API.

```rust
#[mry::mry]
impl Into<&'static str> for Cat {
    fn into(self) -> &'static str {
        self.name
    }
}
```

You can do `cat.mock_into()` as well as `cat.mock_meow()`.

### Mocking a trait with generics or associated type

We can also mock a trait by manually creating a mock struct.
If the trait has a generics or associated type, we need to use this way for now.

```rust
#[mry::mry]
#[derive(Default)]
struct MockIterator {}

#[mry::mry]
impl Iterator for MockIterator {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}
```
