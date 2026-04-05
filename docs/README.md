# EmberFlow Documentation

EmberFlow is a local-first runtime and MCP surface for agentic workflow tracking. It owns canonical track state, runtime events, projections, and the project-level `.emberflow/` state root. The pages here explain the product boundary, the state model, the transport surface, and how agent systems integrate with it.

## Start here

Read these pages in order:

1. [`state-model.md`](state-model.md) — where EmberFlow state lives, how
   `canonical` and `projected` modes differ, and how the shared project root is
   resolved
2. [`projections.md`](projections.md) — how optional filesystem projections
   are modeled and how the projection engine keeps them aligned
3. [`architecture.md`](architecture.md) — what EmberFlow owns, what it does
   not own, and the durable object model
4. [`../mcp/README.md`](../mcp/README.md) — the local stdio MCP surface and
   what it exposes to consumers
5. [`integration.md`](integration.md) — how agent systems integrate with
   EmberFlow without re-creating the state model

## Topic map

### State and storage
- [`state-model.md`](state-model.md) — canonical vs projected modes, root
  resolution, and the `.emberflow/` layout

### Projection lifecycle
- [`projections.md`](projections.md) — projection records and filesystem
  refresh behavior

### Product boundaries
- [`architecture.md`](architecture.md) — EmberFlow's responsibilities and
  object model

### Surface and transport
- [`../mcp/README.md`](../mcp/README.md) — initialization, capabilities,
  canonical track access, and the stdio transport boundary

### Integration
- [`integration.md`](integration.md) — generic agent integration patterns for
  any agent framework or orchestration layer

## If you are looking for...

- **Where the shared database lives** → [`state-model.md`](state-model.md)
- **How projections stay in sync** → [`projections.md`](projections.md)
- **What EmberFlow does** → [`architecture.md`](architecture.md)
- **How a consumer talks to EmberFlow** → [`../mcp/README.md`](../mcp/README.md)
- **How an agent system plugs in** → [`integration.md`](integration.md)

## See Also

- [`../README.md`](../README.md) — project entry point
