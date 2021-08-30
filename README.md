# PPS -- Python Package Index Search

Search the PyPI for packages by name, which was done by `pip search` in the past.

![Demo](./demo.gif)

This was heavily inspired by [pip_search](https://github.com/victorgarric/pip_search) which looks fantastic but is a little slow. 
Also the output is unnecessary hard to process by other command line tools.


## Features
  * Search the Python Package Index by package name
  * show download statistics
  * sort by name, release date, or number of downloads

## Installation

`cargo install pps`

To keep muscle memory in place, you can use
```bash
 alias pip='function _pip(){
    if [ $1 = "search" ]; then
        pps "$2";
    else pip "$@";
    fi;
    };_pip
```
(credits to [pip_search](https://github.com/victorgarric/pip_search))
