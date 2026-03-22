# Get all users
GET https://jsonplaceholder.typicode.com/users
HTTP 200
[Asserts]
jsonpath "$" count == 10

# Get user by ID
GET https://jsonplaceholder.typicode.com/users/1
HTTP 200
[Asserts]
jsonpath "$.name" == "Leanne Graham"
jsonpath "$.email" matches /^[^@]+@[^@]+\.[^@]+$/
