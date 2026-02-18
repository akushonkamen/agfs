# GPTFS Plugin - Async GPT Processing over Persistent Storage

This plugin provides an asynchronous interface to OpenAI-compatible APIs
with persistent file storage using LocalFS.

## Path Layout

```bash
/agents/gptfs/
  inbox/                    # Write any file here to trigger API calls
    request.json           # Example: OpenAI request -> request_response.txt
    prompt.txt             # Example: Text prompt -> prompt_response.txt
    query.md               # Example: Markdown query -> query_response.txt
  outbox/
    request_response.txt   # Response for request.json
    request_status.json    # Status for request.json
    prompt_response.txt    # Response for prompt.txt
    prompt_status.json     # Status for prompt.txt
    query_response.txt     # Response for query.md
    query_status.json      # Status for query.md
```

## Workflow
1. Write any file to the gptfs mount path (e.g., inbox/request.json)
2. File write returns immediately (async processing)
3. Monitor outbox/{filename}_status.json for progress
4. Read response from outbox/{filename}_response.txt when complete

## Example
```bash
# Write an OpenAI request
echo '{"model":"gpt-4o-mini","messages":[{"role":"user","content":"Say hello"}]}' > inbox/request.json
# -> Creates outbox/request_response.txt and outbox/request_status.json

# Write a text prompt
echo "Tell me a joke" > inbox/prompt.txt
# -> Creates outbox/prompt_response.txt and outbox/prompt_status.json

# Write multiple requests concurrently
echo "What is AI?" > inbox/qa1.txt
echo "What is ML?" > inbox/qa2.txt
# -> Creates separate response and status files for each
```

## Configuration

- `api_host`: OpenAI-compatible endpoint
- `api_key`: API authorization key
- `data_dir`: Persistent storage directory
- `workers`: Concurrent API workers (default: 3)
- `mount_path`: Virtual mount path

## Features
- Asynchronous processing (non-blocking writes)
- Persistent storage using LocalFS
- Real-time job status tracking
- Automatic retry with exponential backoff
- Multiple concurrent workers
- Detailed error handling and logging

## License

Apache License 2.0
