# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This source code is dual-licensed under either the MIT license found in the
# LICENSE-MIT file in the root directory of this source tree or the Apache
# License, Version 2.0 found in the LICENSE-APACHE file in the root directory
# of this source tree. You may select, at your option, one of the
# above-listed licenses.

# pyre-strict


from buck2.tests.e2e_util.api.buck import Buck
from buck2.tests.e2e_util.buck_workspace import buck_test
from buck2.tests.e2e_util.helper.utils import filter_events


async def _registered_dispatched_materialized(
    buck: Buck,
) -> tuple[set[str], set[str], set[str]]:
    """Returns (registered, eagerly_dispatched, materialized) path sets from events."""
    register_events = await filter_events(
        buck,
        "Event",
        "data",
        "Instant",
        "data",
        "MaterializerCommand",
        "data",
        "RegisterEagerPaths",
    )
    dispatch_events = await filter_events(
        buck,
        "Event",
        "data",
        "Instant",
        "data",
        "MaterializerCommand",
        "data",
        "EagerDispatchOnDeclare",
    )
    mat_end_events = await filter_events(
        buck,
        "Event",
        "data",
        "SpanEnd",
        "data",
        "Materialization",
    )
    registered = {p for ev in register_events for p in ev["paths"]}
    eagerly_dispatched = {ev["path"] for ev in dispatch_events}
    materialized = {ev["path"] for ev in mat_end_events if ev.get("success")}
    return registered, eagerly_dispatched, materialized


@buck_test(data_dir="combined")
async def test_combined_inputs_register_dispatch_materialize(buck: Buck) -> None:
    """Tests eager materialization with regular, projected, and tset inputs."""
    await buck.build("//:consumer")

    (
        registered,
        eagerly_dispatched,
        materialized,
    ) = await _registered_dispatched_materialized(buck)
    found_basenames = {p.rsplit("/", 1)[-1] for p in registered}

    assert "regular_out" in found_basenames
    assert "out_dir" in found_basenames
    assert not any("/out_dir/file_a" in p for p in registered)

    for tset_member in ("leaf_a_out", "leaf_b_out", "node_x_out"):
        assert tset_member in found_basenames

    assert len(registered) == 5
    assert registered == eagerly_dispatched

    missing = registered - materialized
    assert not missing


@buck_test(data_dir="combined")
async def test_content_based_path_eager_materialization(buck: Buck) -> None:
    """Tests eager materialization with content-based paths."""
    await buck.build("//:consumer_content")

    (
        registered,
        eagerly_dispatched,
        materialized,
    ) = await _registered_dispatched_materialized(buck)

    # Registered cfg-based path but eagerly materialized but cfg-based and content-based paths.
    assert len(registered) == 1
    assert len(eagerly_dispatched) == 2
    assert len(materialized) == 2
