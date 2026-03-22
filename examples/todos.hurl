# Get all todos
GET https://jsonplaceholder.typicode.com/todos/
HTTP 200
[Asserts]
header "Content-Type" contains "application/json"
jsonpath "$[*]" count >= 100

# Get single todo
GET https://jsonplaceholder.typicode.com/todos/1
HTTP 200
[Asserts]
jsonpath "$.userId" == 1
jsonpath "$.completed" isBoolean
