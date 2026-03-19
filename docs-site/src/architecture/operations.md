# Operations & CRDT

All document mutations go through the `Operation` enum:

```rust
enum Operation {
    InsertNode { parent_id, index, node },
    DeleteNode { target_id },
    MoveNode { target_id, new_parent_id, new_index },
    InsertText { target_id, offset, text },
    DeleteText { target_id, offset, length },
    SetAttributes { target_id, attributes },
    RemoveAttributes { target_id, keys },
    SetMetadata { key, value },
    SetStyle { style },
    RemoveStyle { style_id },
}
```

## Undo/Redo

Every operation produces an inverse. Applying the inverse undoes the change:

```
InsertNode → inverse: DeleteNode (with snapshot)
InsertText → inverse: DeleteText
SetAttributes → inverse: SetAttributes (with previous values)
```

## Transactions

Multiple operations can be grouped into a Transaction for atomic undo:

```rust
let mut txn = Transaction::with_label("Bold selection");
txn.push(Operation::set_attributes(run1, bold_attrs));
txn.push(Operation::set_attributes(run2, bold_attrs));
doc.apply_transaction(&txn)?;
// Single undo reverts both
```

## CRDT (Fugue)

The CRDT layer (`s1-crdt`) uses the Fugue algorithm for text:
- Each character has a unique ID + origin references
- Concurrent inserts produce deterministic interleaving
- State vectors track what each replica has seen
- Convergence guaranteed regardless of operation order
