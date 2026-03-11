# S3FS Plugin - AWS S3-backed File System

This plugin provides a file system backed by AWS S3 object storage.

## Features
- Store files and directories in AWS S3
- Support for S3-compatible services (MinIO, LocalStack, etc.)
- Full POSIX-like file system operations
- Automatic directory handling
- Optional key prefix for namespace isolation

## Dynamic Mounting With AGFS Shell

Interactive shell - AWS S3:
```bash
agfs:/> mount s3fs /s3 bucket=my-bucket region=us-east-1 access_key_id=AKIAIOSFODNN7EXAMPLE secret_access_key=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY
agfs:/> mount s3fs /backup bucket=backup-bucket region=us-west-2 access_key_id=YOUR_KEY secret_access_key=YOUR_SECRET prefix=myapp/
```

Interactive shell - S3-compatible (MinIO):
```bash
agfs:/> mount s3fs /minio bucket=my-bucket region=us-east-1 access_key_id=minioadmin secret_access_key=minioadmin endpoint=http://localhost:9000 disable_ssl=true
```

Direct command - AWS S3:
```bash 
uv run agfs mount s3fs /s3 bucket=my-bucket region=us-east-1 access_key_id=AKIAIOSFODNN7EXAMPLE secret_access_key=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY
```

Direct command - MinIO:
```bash
uv run agfs mount s3fs /minio bucket=test region=us-east-1 access_key_id=minioadmin secret_access_key=minioadmin endpoint=http://localhost:9000 disable_ssl=true
```

## Configuration Parameters

### Required:
- `bucket`: S3 bucket name
- `region`: AWS region (e.g., `us-east-1`, `eu-west-1`)
- `access_key_id`: AWS/S3 access key ID
- `secret_access_key`: AWS/S3 secret access key

### Optional:
- `prefix`: Key prefix for namespace isolation (e.g., `myapp/`)
- `endpoint`: Custom S3 endpoint for S3-compatible services (e.g., MinIO)
- `disable_ssl`: Set to true to disable SSL for local services (default: false)

### Examples
```bash  
# Multiple buckets with different configurations
agfs:/> mount s3fs /s3-prod bucket=prod-bucket region=us-east-1 access_key_id=KEY1 secret_access_key=SECRET1
agfs:/> mount s3fs /s3-dev bucket=dev-bucket region=us-west-2 access_key_id=KEY2 secret_access_key=SECRET2 prefix=dev/
```

## Static Configuration (config.yaml):

Alternative to dynamic mounting - configure in server config file:

AWS S3:
```yaml
plugins:
  s3fs:
    enabled: true
    path: /s3fs
    config:
      region: us-east-1
      bucket: my-bucket
      access_key_id: AKIAIOSFODNN7EXAMPLE
      secret_access_key: wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY
      prefix: agfs/ # Optional: all keys will be prefixed with this
```

S3-Compatible Service (MinIO, LocalStack):
```yaml
plugins:
  s3fs:
    enabled: true
    path: /s3fs
    config:
      region: us-east-1
      bucket: my-bucket
      access_key_id: minioadmin
      secret_access_key: minioadmin
      endpoint: http://localhost:9000
      disable_ssl: true
```

Multiple S3 Buckets:
```yaml
plugins:
  s3fs:
    - name: prod
      enabled: true
      path: /s3/prod
      config:
        region: us-east-1
        bucket: production-bucket
        access_key_id: "..."
        secret_access_key: "..."
    - name: dev
      enabled: true
      path: /s3/dev
      config:
        region: us-west-2
        bucket: development-bucket
        access_key_id: "..."
        secret_access_key: "..."
```

## Usage

Create a directory
```bash    
agfs mkdir /s3fs/data
```

Create a file
```bash
agfs write /s3fs/data/file.txt "Hello, S3!"
```

Read a file:
```bash
agfs cat /s3fs/data/file.txt
```

List directory:
```bash
agfs ls /s3fs/data
```

Remove file:
```bash
agfs rm /s3fs/data/file.txt
```

Remove directory (must be empty):
```bash
agfs rm /s3fs/data
```

Remove directory recursively:
```bash
agfs rm -r /s3fs/data
```

## Example

```bash
# Basic file operations
agfs:/> mkdir /s3fs/documents
agfs:/> echo "Important data" > /s3fs/documents/report.txt
agfs:/> cat /s3fs/documents/report.txt
```  

Important data
```bash
# List contents
agfs:/> ls /s3fs/documents
report.txt

# Move/rename
agfs:/> mv /s3fs/documents/report.txt /s3fs/documents/report-2024.txt
```

## Notes
- S3 doesn't have real directories; they are simulated with `/` in object keys
- Large files may take time to upload/download
- Permissions (`chmod`) are not supported by S3
- Atomic operations are limited by S3's eventual consistency model

## Use Case
- Cloud-native file storage
- Backup and archival
- Sharing files across distributed systems
- Cost-effective long-term storage
- Integration with AWS services

## Advantages
- Unlimited storage capacity
- High durability (99.999999999%)
- Geographic redundancy
- Pay-per-use pricing
- Versioning and lifecycle policies (via S3 bucket settings)

## License

Apache License 2.0
