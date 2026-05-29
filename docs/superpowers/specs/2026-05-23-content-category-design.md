# Content Category and Custom Rules Design Specification

Add `content` as a new first-class project category in `projm` and enable detecting projects with specific dependencies (like `remotion`) to place them inside the `content/` folder.

## Overview

In `projm`, projects are automatically scanned, classified, and organized into specific physical directories based on their type (e.g. `apps`, `services`, `ui`, etc.). To support developer content creation projects (such as those using React/Remotion to generate programmatic videos), we introduce `content` as a first-class project category.

This design specification details the integration of the new `content` category in `projm` and how custom dependency-based matching rules (such as `remotion`) can route projects to it.

## Proposed Changes

### 1. `Category` Enum & Associated Traits (`src/classify.rs`)
* Add `Category::Content` to the `Category` enum.
* Implement directory name mapping in `dir_name()`:
  ```rust
  Self::Content => "content"
  ```
* Implement a visual, high-quality color for terminal display in `label()`:
  ```rust
  Self::Content => s.truecolor(255, 105, 180).bold().to_string(), // Hot Pink
  ```
* Register `Self::Content` in the `all()` method list.

### 2. Rule Parsing & Default Template (`src/rules.rs`)
* Update `parse_category` to recognize `"content"` as a valid category mapping to `Some(Category::Content)`.
* Update the default template in `init_default_rules()` to:
  * Document `content` under the list of allowed categories.
  * Add a commented-out rule example detecting the `remotion` dependency.

### 3. Tests Update (`tests/classify_tests.rs`)
* Update `category_dir_names_are_stable()` to assert `Category::Content.dir_name() == "content"`.
* Update `category_all_has_seven_variants()` to `category_all_has_eight_variants()` and verify that the length of `Category::all()` is indeed 8.
* Add a test case `classify_rule_has_dep_remotion_to_content` that:
  * Creates a temporary project with a `package.json` containing `remotion` in its dependencies.
  * Runs the classifier with a custom rule `{ has_dep: Some("remotion"), category: Category::Content, ... }`.
  * Verifies the resulting classification is `Category::Content`.

### 4. Context Documentation (`CONTEXT.md`)
* Update `Category` description from "seven" to "eight" predefined, mutually exclusive project types and add `content` to the list.

## Verification Plan

### Automated Tests
* Run `cargo test` to execute all classification, rule parsing, and category checks.
