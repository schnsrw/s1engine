# Document Model

The document model (`s1-model`) is a tree of nodes:

```
Document
  └─ Body
       ├─ Paragraph
       │    ├─ Run (formatting container)
       │    │    ├─ Text ("Hello ")
       │    │    └─ Text ("world")
       │    └─ Run (bold)
       │         └─ Text ("bold text")
       ├─ Table
       │    ├─ TableRow
       │    │    ├─ TableCell
       │    │    └─ TableCell
       │    └─ TableRow
       └─ Paragraph
```

## Node Types

| Type | Description | Can Contain |
|------|-------------|-------------|
| Document | Root node | Body, Header, Footer, CommentBody |
| Body | Main content | Paragraph, Table, TOC, PageBreak |
| Paragraph | Text container | Run, Image, LineBreak, Tab, Field |
| Run | Formatting span | Text |
| Text | Leaf text content | (none) |
| Table | Grid container | TableRow |
| TableRow | Row | TableCell |
| TableCell | Cell | Paragraph |
| Image | Inline image | (none) |

## NodeId

Every node has a globally unique `NodeId(replica_id, counter)`:
- `replica_id` identifies the creating peer (for CRDT)
- `counter` is monotonically increasing per replica

## Attributes

Nodes carry an `AttributeMap` of key-value formatting:
- Character: Bold, Italic, FontSize, FontFamily, Color
- Paragraph: Alignment, SpacingBefore, SpacingAfter, IndentLeft
- Image: ImageWidth, ImageHeight, ImageMediaId
- Style: StyleId references a named Style definition
