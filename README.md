# rbook
## Experimental Asynchronous Implementation Using Generics
- Completely additive (`async` feature)
- Provides compile-time type-safety between synchronous and asynchronous variants.

Synchronous:
```rust
fn main() -> Result<()> {
    let epub = Epub::open("path/to/file")?;
    let reader = epub.reader();
    
    for entry in epub.manifest().entries() {
        entry.read_bytes()?;
    }

    epub.read_resource_bytes("c1")?;
    
    while let Some(Ok(content)) = reader.read_next() { /*...*/ }
    
    Ok(())
}
```
Asynchronous:
```rust
async fn main() -> Result<()> {
    let epub = AsyncEpub::open("path/to/file").await?;
    let reader = epub.reader();

    for entry in epub.manifest().entries() {
        entry.read_bytes().await?;
    }

    epub.read_resource_bytes("c1").await?;

    while let Some(Ok(content)) = reader.read_next().await { /*...*/ }

    Ok(())
}
```

## Some Notes

### Strengths
#### 1. Versatile
`rbook` becomes more versatile; compatible and cohesive with async rust.
This can boost performance in async environments as blocking calls are not needed.

#### 2. Nearly Non-Existent Duplication
Generics greatly avoid duplication between sync and async implementations; 
nearly non-existent, providing shared behavior between sync and async variants.
As a result, only the portions that are truly async are primarily worked on.

### Concerns
#### 1. Generics
While async compatibility is great, the ergonomics of the library is reduced, introducing a steeper learning curve.
Most concrete instances now require generics to specify whether the instance should support 
synchronous or asynchronous methods.

For example:
```
Epub                -> Epub<A>
EpubManifest<'a>    -> EpubManifest<'a, A>
EpubSpineEntry<'a>  -> EpubSpineEntry<'a, A>
EpubReader<'a>      -> EpubReader<'a, A>
```

`A` indicates the "async-ness". 
Basically, it dictates which methods to use. 
In the current implementation, `A` is one of the following:
- `SynchronousArchive`/`&SynchronousArchive` (Use synchronous methods)
  > Note: Didn't name it "SyncArchive" to avoid confusion with the "Sync" trait.
- `AsyncArchive`/`&AsyncArchive` (Use asynchronous methods)
  > Note: Maybe rename to "AsynchronousArchive"...

The verbose generics can be alleviated by using type aliases and renaming the concrete type:
```
type Epub                   = EpubData<SynchronousArchive>;
type AsyncEpub              = EpubData<AsyncArchive>;
type EpubManifest<'a>       = EpubManifestData<'a, &'a SynchronousArchive>;
type AsyncEpubManifest<'a>  = EpubManifestData<'a, &'a AsyncArchive>;
```

#### 2. Method Parameters & Struct Fields

Due to generics, using the provided rbook types as parameters or as struct fields is tedious:

**Before**:
```rust
// Access to data + sync IO
fn process_entry(entry: EpubManifestEntry) {
    let id = entry.id();
    let bytes = entry.read_bytes();
}
```
**After**:
- Users must now be aware of generic arguments and trait bounds.
```rust
// Access to data - no IO
fn process_entry<A: Copy>(entry: EpubManifestEntry<A>) {
    // Note - A: Copy is leaky - Will replace with a marker trait
    let id = entry.id();
    // let bytes = entry.read_bytes(); No IO available
}
// Access to data + sync IO
fn process_entry(entry: EpubManifestEntry<&SynchronousArchive>) {
    let bytes = entry.read_bytes();
}
// Access to data + async IO
async fn process_entry(entry: EpubManifestEntry<&AsyncArchive>) {
    let bytes = entry.read_bytes().await;
}
```

Descriptive yet concise documentation most likely can get around this problem. 
However, as shown above, it's clearly not as intuitive as it was before.

I stuck to a general pattern to make things easier to reason about generic structs, 
which can be improved upon:
- Generic structs that do **not** have a lifetime (`Struct<A>`) use:
  - `A: ArchiveLike` Trait bound for non-IO functionality
  - `SynchronousArchive` (for sync IO functionality)
  - `AsyncArchive` (for async IO functionality)
- Generic structs that **have** a lifetime (`Struct<'a, A>`) use:
  - `A: Copy` Trait bound for non-IO functionality 
  - `&SynchronousArchive` (for sync IO functionality)
  - `&AsyncArchive` (for async IO functionality)

For example, if I want `EpubData<A>` as a parameter, then:
- `EpubData<A> where A: ArchiveLike` 
  for non-IO functionality (e.g., view metadata, fields, etc.)
- `EpubData<SynchronousArchive>` 
  for accessing synchronous IO functionality (e.g. such as reading bytes from a files)
- `EpubData<AsyncArchive>`
  for accessing asynchronous IO functionality (similar to SynchronousArchive)

Likewise for `EpubManifestEntry<'a, A>`:
- `EpubManifestEntry<A> where A: Copy`
- `EpubManifestEntry<&SynchronousArchive>`
- `EpubManifestEntry<&AsynchronousArchive>`

### Overall Thoughts

While it works and the architecture is sound, 
the design primarily conflicts with the requirements I have established for `rbook`.
> A fast, format-agnostic, ergonomic ebook library with a focus on EPUB.
  The primary goal of `rbook` is to provide an easy-to-use high-level API for handling ebooks. 
  Most importantly, this library is designed with future formats in mind (`CBZ`, `FB2`, `MOBI`, etc.) 
  via core traits defined within the `ebook` and `reader` module, allowing all formats to share the same "base" API.

The generic-based async/sync design achieves unified implementations with almost no duplication, 
at the cost of exposing generics across the public API.
Simple function parameters become verbose unless type aliases are heavily used.
This approach is not intuitive and reduces overall ergonomics, making `rbook` harder to use.

However... the design can be improved, albeit in its current form, it does not align well with the ergonomics goal.
 
---

Design 1 of 2 to introduce async support.
