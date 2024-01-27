### Asset registry

## Overview
Asset registry provides functionality to create, store and keep tracking of existing assets in a system.

### Terminology

- **CoreAssetId** - asset id of native/core asset. Usually 0.
- **NextAssetId** - asset id to be assigned for next asset added to the system. 
- **AssetIds** - list of existing asset ids
- **AssetDetail** - details of an asset such as type, name or whether it is locked or not.
- **AssetMetadata** - additional optional metadata of an asset ( symbol, decimals)
- **AssetLocation** - information of native location of an asset. Used in XCM.

### Implementation detail

For each newly registered asset, a sequential id is assigned to that asset. This id identifies the asset and can be used directly in transfers or any other operation which works with an asset ( without performing any additioanl asset check or asset retrieval).

There is a mapping between the name and asset id stored as well, which helps and is used in AMM Implementation where there is a need to register a pool asset and only name is provided ( see `get_or_create_asset` ).

An asset has additional details stored on chain such as name and type. 

Optional metadata can be also set for an asset.

The registry pallet supports storing of native location of an asset. This can be used in XCM where it is possible to create mapping between native location and local system asset ids. 

### Interface
- `get_or_create_asset` - creates new asset id for give asset name. If such asset already exists, it returns the corresponding asset id.


# NFT-pallet
## Introduction
The NFT Pallet is a module that allows developers to integrate Non-Fungible Tokens into their blockchain applications. NFTs are unique digital assets that can represent ownership of digital or physical items, such as artwork, collectibles, or in-game items.

## Features
- Token Creation: Easily create unique NFTs with customizable metadata.
- Ownership Management: Transfer ownership of NFTs between accounts.
- Metadata Storage: Store metadata associated with each NFT.
- Querying: Retrieve information about NFTs and their owners.
- Extensibility: Designed to be integrated into existing blockchain applications.

## Getting Started
### Prerequisites
- Rust: Make sure you have Rust and the necessary toolchain installed. You can install it using [rustup](https://rustup.rs/).

