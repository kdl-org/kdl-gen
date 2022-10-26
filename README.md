kdl-gen
=======

**A KDL Document Generator**

Generates random KDL documents to assist in fuzzing parsers.
All documents generated should be valid KDL according to the specification 
at [kdl.dev](https://kdl.dev).


Running
=======

If run without arguments, will first print the seed being used to stderr then a large
KDL document to stdout.

A number of arguments are available to control the shape of the output:

```
-d, --depth-max <DEPTH_MAX>                      [default: 3]
-n, --nodes-per-child-max <NODES_PER_CHILD_MAX>  [default: 3]
-e, --extra-space-max <EXTRA_SPACE_MAX>          [default: 3]
-p, --props-or-args-max <PROPS_OR_ARGS_MAX>      [default: 10]
-b, --blank-lines-max <BLANK_LINES_MAX>          [default: 1]
-i, --identifier-len-max <IDENTIFIER_LEN_MAX>    [default: 20]
-s, --string-len-max <STRING_LEN_MAX>            [default: 100]
-l, --num-len-max <NUM_LEN_MAX>                  [default: 10]
-c, --comment-len-max <COMMENT_LEN_MAX>          [default: 100]
-a, --ascii-only                                 [default: false]
```

In addition, several arguments are available for help debugging when
a parser fails on a document. The `--debug` flag will cause the 
generator to print tags indicating when it is entering or exiting a
rule. Note that this will cause the output to no longer be valid KDL.
The `--seed <u64 int>` argument will generate an identical document 
to a previous run so long as all other options other than `--debug`
are the same.
