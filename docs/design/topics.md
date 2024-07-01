# Topics

Authors: [Philip Metzger](mailto:philipmetzger@bluewin.ch), [Noah Mayr](mailto:dev@noahmayr.com)
 [Anton Bulakh](mailto:him@necaq.ua)

## Summary

Introduce Topics as a truly Jujutsu native way for topological branches, which 
also replace the current bookmark concept for Git interop. As they have been
documented to be confusing users coming from Git. They also replace the 
`[experimental-advance-branches]` config for those who currently use it, as 
such a behavior will be built-in for Topics.

## Prior work 

Currently there only is Mercurial which has a implementation of 
[Topics][hg-topic]. There also is the [Topic feature][gerrit-topics] in Gerrit,
which groups commits with a single identifier.


## Goals and non-goals

### Goals

The goals for this Project are small, see below.

* Introduce the concept of topological branches for Jujutsu.
* Simplify Git interop by reducing the burden on `jj bookmark`.
* Add Change metadata as a concept.
* Remove the awkward `bookmark` to Git `branch` mapping.

### Non-Goals

* TODO

## Overview



### Detailed Design


#### Storage

We should store `Topics` as metadata on the serialized proto, without 
considering the resulting Gencode. 



```protobuf
// A simple Key-Value pair. 
message StringPair {
  string key = 1;
  string value = 2;
  // This `Any` is optional to attach further backend specific metadata on it.
  // optional google.protobuf.Any any_value = 3;
}

message Commit {
  //...
  repeated StringPair metadata = N;
}
```

while the actual code should look like this:

```rust
#[derive(ContentHash, ...)]
struct Commit {
  //...
  #[ContentHash(ignore = true)]
  topics: HashMap<String, String>
}
```

#### Backend implications

If Topics were stored as commit metadata, it would allow backends to drop 
the metadata if necessary. This property can be useful to mark tests as passing
on a specific client. 

For the Git backend, we 

## Alternatives considered 

### Store Topics out-of-band (Not directly on the Commit)

See [#1889][prototype] for another idea of keeping the topics out of band.
While it works, it no longer


### Single Head Topics

While these are conceptually simpler, they wouldn't help with Git interop where
it is useful to map a single underlying to multiple Git branches. This also 
worsens the `jj`-`Git` interop story.

## Future Possibilities

In the future we could attach a `google.protobuf.Any` to the Change metadata, 
which would allow specific clients, such as testrunners to directly attach test
results which could be neat. 

[hg-topics]:
[gerrit-topics]: 
