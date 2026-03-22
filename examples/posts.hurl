# Get all posts
GET https://jsonplaceholder.typicode.com/posts
HTTP 200
[Asserts]
jsonpath "$" count == 100

# Get single post
GET https://jsonplaceholder.typicode.com/posts/1
HTTP 200
[Asserts]
jsonpath "$.id" == 1
jsonpath "$.title" exists
