GET {{base_url}}/api/files?path=examples
HTTP 200
[Asserts]
jsonpath "$.children" count == 4
jsonpath "$.children" exists
jsonpath "$.children[0]" exists
jsonpath "$.name" == "examples"
jsonpath "$.relative_path" == ""
jsonpath "$.type" == "folder"

GET {{base_url}}/api/file?path=examples%2Fusers.hurl
HTTP 200

POST {{base_url}}/api/parse
Content-Type: application/json
{
  "content": "GET https://jsonplaceholder.typicode.com/todos/1\nHTTP 200"
}
HTTP 200

POST {{base_url}}/api/run-entry
Content-Type: application/json
{
  "content": "GET https://jsonplaceholder.typicode.com/todos/1\nHTTP 200",
  "entry_index": 0,
  "env": {
    "sample": "value"
  }
}
HTTP 200

POST {{base_url}}/api/run-to-end
Content-Type: application/json
{
  "content": "GET https://jsonplaceholder.typicode.com/todos/1\nHTTP 200",
  "entry_index": 0,
  "env": {
    "sample": "value"
  }
}
HTTP 200
[Asserts]
jsonpath "$.results" count == 1
jsonpath "$.results" exists
jsonpath "$.results[0]" exists

POST {{base_url}}/api/run-from-begin
Content-Type: application/json
{
  "content": "GET https://jsonplaceholder.typicode.com/todos/1\nHTTP 200",
  "entry_index": 0,
  "env": {
    "sample": "value"
  }
}
HTTP 200

POST {{base_url}}/api/run-selection
Content-Type: application/json
{
  "content": "GET https://jsonplaceholder.typicode.com/todos/1\nHTTP 200",
  "selection": {
    "start_line": 1,
    "end_line": 2
  },
  "env": {
    "sample": "value"
  }
}
HTTP 200

POST {{base_url}}/api/run-file
Content-Type: application/json
{
  "content": "GET https://jsonplaceholder.typicode.com/todos/1\nHTTP 200",
  "env": {
    "sample": "value"
  }
}
HTTP 200

POST {{base_url}}/api/test-file
Content-Type: application/json
{
  "content": "GET https://jsonplaceholder.typicode.com/todos/1\nHTTP 200",
  "env": {
    "sample": "value"
  }
}
HTTP 200

POST {{base_url}}/api/rerun-last
HTTP 200

POST {{base_url}}/api/cancel
Content-Type: application/json
{
  "run_id": "example-run-id"
}
HTTP 400