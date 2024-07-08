<!--
Guiding Principles:

Changelogs are for humans, not machines.
There should be an entry for every single version.
The same types of changes should be grouped.
Versions and sections should be linkable.
The latest version comes first.
The release date of each version is displayed.
Mention whether you follow Semantic Versioning.

Usage:

Change log entries are to be added to the Unreleased section under the
appropriate stanza (see below). Each entry should ideally include a tag and
the Github issue reference in the following format:

* (<tag>) \#<issue-number> message

The issue numbers will later be link-ified during the release process so you do
not have to worry about including a link manually, but you can if you wish.

Types of changes (Stanzas):

"Features" for new features.
"Improvements" for changes in existing functionality.
"Deprecated" for soon-to-be removed features.
"Bug Fixes" for any bug fixes.
"API Breaking" for breaking exported APIs used by developers building on SDK.
Ref: https://keepachangelog.com/en/1.0.0/
-->

# Changelog

## Unreleased - v0.2.1 Target

[Tracking](https://github.com/skip-mev/skip-api-contracts/issues/49)

### Minor Documentation Improvements
- [#52](https://github.com/skip-mev/skip-api-contracts/pull/52) Ensure Skip package types, methods, and functions are well documented.

### Minor Code Improvements
- [#53](https://github.com/skip-mev/skip-api-contracts/pull/53) Consolidate Neutron and Osmosis QueryMsg enum's into one enum.
- [#63](https://github.com/skip-mev/skip-api-contracts/pull/63) Refactor Fee Swap Into Ibc Transfer Variant of Action Enum

### Minor Testing Improvements
- [#55](https://github.com/skip-mev/skip-api-contracts/pull/55) Add unit tests to methods and functions in Skip packages folder.

### Minor Workspace Improvements
- [#57](https://github.com/skip-mev/skip-api-contracts/pull/57) Adds cargo upgrade to Makefile, runs cargo upgrade and cargo update.
- [#58](https://github.com/skip-mev/skip-api-contracts/pull/58) Updates make optimize to target /target instead of /code/target
- [#59](https://github.com/skip-mev/skip-api-contracts/pull/59) Add cargo fmt check in github workflow
- [#60](https://github.com/skip-mev/skip-api-contracts/pull/60) Update github workflow to also run on push to main
- [#61](https://github.com/skip-mev/skip-api-contracts/pull/61) Add Skip package path to workspace cargo.toml
- [#64](https://github.com/skip-mev/skip-api-contracts/pull/64) Add Json Schema Generation
- [#65](https://github.com/skip-mev/skip-api-contracts/pull/65) Restructure Contracts Folder From networks/ to adapters/

## [v0.2.0](https://github.com/skip-mev/skip-api-contracts/releases/tag/v0.2.0) - 2023-08-03

[Tracking](https://github.com/skip-mev/skip-api-contracts/issues/28)

### Notable Features
- [#21](https://github.com/skip-mev/skip-api-contracts/issues/21) Support swap exact out for the user, refunding unused input token back to a refund address on the swap chain.

### Notable Improvements
- [#31](https://github.com/skip-mev/skip-api-contracts/pull/31) Derive user swap coin in from remaining coin received only
- [#32](https://github.com/skip-mev/skip-api-contracts/pull/32) Upgrade cosmwasm_std to 1.3
- [#36](https://github.com/skip-mev/skip-api-contracts/pull/36) Derive Fee Swap Coin Out From IBC Fees Provided
- [#42](https://github.com/skip-mev/skip-api-contracts/pull/42) Upgrade Rustc to 1.71.0, Workspace to 2.0, and CosmWasm Optimizer to 0.14.0

### Notable Other Changes
- [#38](https://github.com/skip-mev/skip-api-contracts/pull/38) Affiliate fee BPS based off of `min_coin` instead of the coin received from the swap itself.
- [#19](https://github.com/skip-mev/skip-api-contracts/pull/19) IBC adapter contracts refund the user based off of querying the contract balance rather than relying on stored variables.
- [#33](https://github.com/skip-mev/skip-api-contracts/pull/33) Replace custom impl of Coins with cw_std Coins 

## [v0.1.0](https://github.com/skip-mev/skip-api-contracts/releases/tag/v0.1.0) - 2023-07-18

Let There Be Skip Go Contracts!

### Features
- Supports swap exact in for the user.
- Supports three post-swap actions:
    - Bank Send
    - IBC Transfer
    - Contract Call
- Supports a fee swap if the post swap action is an IBC transfer and requires IBC fees.
- Supports affiliate fee payments.

### Network / DEX Support
- Osmosis: Poolmanager, Neutron: Astroport