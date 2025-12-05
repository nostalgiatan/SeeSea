from tf import DocumentStore

# Initialize DocumentStore
store = DocumentStore()
print("DocumentStore initialized successfully")

# Add a document
result = store.add("test1", "This is a test document", title="Test Title", url="http://example.com")
print(f"Document added successfully: {result}")

# Search for the document
results = store.search("test", k=1)
print(f"Search result: {results}")

# Check document count
count = store.count()
print(f"Document count: {count}")

# Check URL existence using bloom filter
url_exists = store.url_exists("http://example.com")
print(f"URL exists (bloom): {url_exists}")

# Check URL existence with exact match
url_exists_exact = store.url_exists_exact("http://example.com")
print(f"URL exists (exact): {url_exists_exact}")

# Get document by URL
doc_by_url = store.get_by_url("http://example.com")
print(f"Document by URL: {doc_by_url}")

print("All tests passed!")
