# REST API

The rudra-server provides a REST API for document management.

## Base URL

```
http://localhost:8080/api/v1/
```

## Endpoints

### Documents

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/documents` | Upload a document (multipart) |
| `GET` | `/documents` | List documents (paginated) |
| `GET` | `/documents/:id` | Get document metadata |
| `GET` | `/documents/:id/content` | Download document bytes |
| `DELETE` | `/documents/:id` | Delete a document |
| `GET` | `/documents/:id/thumbnail` | Get PDF thumbnail |

### Conversion

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/convert` | Stateless format conversion |

### Webhooks

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/webhooks` | Register a webhook |
| `GET` | `/webhooks` | List webhooks |
| `DELETE` | `/webhooks/:id` | Delete a webhook |

### System

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/health` | Health check |
| `GET` | `/info` | Server info |

## Example: Upload and Convert

```bash
# Upload a DOCX
curl -X POST http://localhost:8080/api/v1/documents \
  -F file=@report.docx

# Convert to PDF
curl -X POST http://localhost:8080/api/v1/convert \
  -F file=@report.docx \
  -F format=pdf \
  -o report.pdf
```
