# J1 behavior gate — emitted vs original (same inputs)

## js_01_util — **BEHAVIOR-MATCH**

```
PASS camelCase("data-bar") -> "dataBar"
PASS camelCase("foo_bar_baz") -> "fooBarBaz"
PASS camelCase("foo") -> "foo"
PASS camelCase("foo-") -> "foo"
PASS camelCase("abc.def") -> "abcDef"
PASS cssCase("fooBar") -> "foo-bar"
PASS cssCase("already") -> "already"
PASS cssCase("Foo") -> "-foo"
PASS cssCase("aB") -> "a-b"
PASS isHtml("<div>") -> true
PASS isHtml("<a>") -> true
PASS isHtml("abc") -> false
PASS isHtml("<") -> false
PASS isHtml("<a") -> false
PASS isHtml("<3 heart") -> false
PASS isHtml("<!doctype html>") -> true
PASS isHtml("a < b > c") -> false
PASS isCheerio({"cheerio":"[cheerio object]"}) -> true
PASS isCheerio({"cheerio":"no"}) -> false
PASS isCheerio(null) -> false
PASS isCheerio({}) -> false
PASS domEach(side effects + identity)
RESULTS: ALL MATCH
```
## js_08_options — **BEHAVIOR-MATCH**

```
PASS flattenOptions(null, {"a":1}) -> {"a":1}
PASS flattenOptions({}, null) -> {"_useHtmlParser2":false}
PASS flattenOptions({"xmlMode":true}, null) -> {"_useHtmlParser2":true,"xmlMode":true}
PASS flattenOptions({"xml":true}, null) -> {"_useHtmlParser2":true,"xml":true,"xmlMode":true}
PASS flattenOptions({"xml":{"q":1}}, null) -> {"_useHtmlParser2":true,"xml":{"q":1},"xmlMode":true,"q":1}
PASS flattenOptions({"xml":true,"x":5}, null) -> {"_useHtmlParser2":true,"xml":true,"x":5,"xmlMode":true}
PASS flattenOptions({"xmlMode":false}, null) -> {"_useHtmlParser2":false,"xmlMode":false}
PASS flattenOptions(null, null) -> {"_useHtmlParser2":false}
RESULTS: ALL MATCH
```
## linked_modules — **BEHAVIOR-MATCH**

```
PASS Report(3) -> 15
PASS Report(0) -> 0
PASS Report(10) -> 50
PASS ReportMany([1,2,4]) -> [2,4,8]
PASS ReportMany([]) -> []
RESULTS: ALL MATCH
```
