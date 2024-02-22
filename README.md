Answers the question "How much space would these zip files take if they were extracted onto disk instead of archived?"

Useful if you decided to store files in zip archives for space saving purposes and you aren't sure if that actually was a good decision or not... 😅

## Install

```sh
cargo install --git https://github.com/kj800x/zip-investigator
```

## Use

```sh
zip-investigator /data/path/to/folder/containing/zips
```

## Output:

### Good use of zip files for space savings:

```sh
$ zip-investigator ~/src
...snipped...

File: ./bostonography/A3/world_shape_file.zip
Extracted size : 478.10 kB
Compressed size: 230.65 kB (48.24%)
Savings        : 247.46 kB

File: ./bostonography/A3/Tracts_Boston_2010_BARI.zip
Extracted size : 551.32 kB
Compressed size: 189.72 kB (34.41%)
Savings        : 361.60 kB

File: ./finalproject-theatre-lighting-simulator/common/thirdparty/glm-0.9.8.5.zip
Extracted size : 13.43 MB
Compressed size: 4.34 MB (32.35%)
Savings        : 9.08 MB

Total extracted size : 203442061 (203.44 MB)
Total compressed size: 40366626 (40.37 MB) - (19.84%)
Total savings        : 163075435 (163.08 MB)
```

### Bad use of zip files for space savings:

```
$ zip-investigator /data/archive
...snipped...

File: /data/archive/044902.zip
Extracted size : 10.75 MB
Compressed size: 10.68 MB (99.34%)
Savings        : 70.88 kB

File: /data/archive/0120201.zip
Extracted size : 13.79 MB
Compressed size: 13.60 MB (98.58%)
Savings        : 195.19 kB

File: /data/archive/0180401.zip
Extracted size : 4.23 MB
Compressed size: 4.23 MB (100.02%)
Savings        : 18.45 EB

File: /data/archive/064401.zip
Extracted size : 8.96 MB
Compressed size: 8.97 MB (100.02%)
Savings        : 18.45 EB

Total extracted size : 22874191773 (22.87 GB)
Total compressed size: 22638768579 (22.64 GB) - (98.97%)
Total savings        : 235423194 (235.42 MB)
```

