/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use allocative::Allocative;
use async_trait::async_trait;
use dice::DiceComputations;
use dice::DiceTransactionUpdater;
use dice::InjectedKey;
use dice::PagableValueSerialize;
use dice::ValueSerialize;
use dupe::Dupe;
use pagable::Pagable;
use pagable::pagable_typetag;

#[derive(Debug, Clone, Dupe, Eq, PartialEq, Allocative, Pagable)]
struct StarlarkTypesValue {
    disable_starlark_types: bool,
    unstable_typecheck: bool,
    /// True if buck2 should drive the recursive-descent Starlark parser
    /// instead of the default LALRPOP parser. Stored as a bool here so we
    /// don't have to pull DICE-trait derives (Pagable / Allocative / Dupe)
    /// onto the upstream parser-kind enum; the consumption site converts.
    use_rd_parser: bool,
}

#[derive(
    Debug,
    derive_more::Display,
    Copy,
    Clone,
    Dupe,
    Eq,
    PartialEq,
    Hash,
    Allocative,
    Pagable
)]
#[display("{:?}", self)]
#[pagable_typetag(dice::DiceKeyDyn)]
struct StarlarkTypesKey;

impl InjectedKey for StarlarkTypesKey {
    type Value = StarlarkTypesValue;

    fn equality(x: &StarlarkTypesValue, y: &StarlarkTypesValue) -> bool {
        x == y
    }

    fn value_serialize() -> impl ValueSerialize<Value = Self::Value> {
        PagableValueSerialize::<Self::Value>::new()
    }
}

pub trait SetStarlarkTypes {
    fn set_starlark_types(
        &mut self,
        disable_starlark_types: bool,
        unstable_typecheck: bool,
        use_rd_parser: bool,
    ) -> buck2_error::Result<()>;
}

impl SetStarlarkTypes for DiceTransactionUpdater {
    fn set_starlark_types(
        &mut self,
        disable_starlark_types: bool,
        unstable_typecheck: bool,
        use_rd_parser: bool,
    ) -> buck2_error::Result<()> {
        Ok(self.changed_to([(
            StarlarkTypesKey,
            StarlarkTypesValue {
                disable_starlark_types,
                unstable_typecheck,
                use_rd_parser,
            },
        )])?)
    }
}

#[async_trait]
pub trait GetStarlarkTypes {
    async fn get_disable_starlark_types(&mut self) -> buck2_error::Result<bool>;
    async fn get_unstable_typecheck(&mut self) -> buck2_error::Result<bool>;
    async fn get_use_rd_parser(&mut self) -> buck2_error::Result<bool>;
}

#[async_trait]
impl GetStarlarkTypes for DiceComputations<'_> {
    async fn get_disable_starlark_types(&mut self) -> buck2_error::Result<bool> {
        Ok(self
            .compute(&StarlarkTypesKey)
            .await?
            .disable_starlark_types)
    }

    async fn get_unstable_typecheck(&mut self) -> buck2_error::Result<bool> {
        Ok(self.compute(&StarlarkTypesKey).await?.unstable_typecheck)
    }

    async fn get_use_rd_parser(&mut self) -> buck2_error::Result<bool> {
        Ok(self.compute(&StarlarkTypesKey).await?.use_rd_parser)
    }
}
