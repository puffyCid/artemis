import argparse
from pathlib import Path
import yaml
import tomli_w

'''
Scan the provided folder recursively until we find all tkape files
'''
def recurseFolder(folder, output, enable, quiet): 
    for entry in folder.iterdir():
        # Ignore the !Disabled folder by default
        if entry.name == "!Disabled" and not enable:
            continue
        # Module Kape files are not supported yet
        if entry.is_file() and entry.suffix == ".tkape":
            parseTKape(entry, quiet, output)
            continue
        if not entry.is_dir():
            continue
        
        recurseFolder(entry, output, enable, quiet)

'''
Scan the provided folder recursively until we find a specific tkape file
Used to bundle Compound Kape collections
'''
def findKapeFile(folder, filename):
    for entry in folder.iterdir():
        # The Kape file names are case sensitive lol
        if str(entry.name).lower() == filename.lower():
            return entry
        if not entry.is_dir():
            continue
        result = findKapeFile(entry, filename)
        if result != None:
            return result

'''
Parse the Kape file and convert to TOML file
A Kape file is just a YAML file
'''
def parseTKape(kape, quiet, output=None):
    with open(kape, 'r') as file:
        data = yaml.load(file, Loader=yaml.SafeLoader)
    meta = {
        'Description': data.get('Description'),
        'Author': data.get('Author'),
        'Version': data.get('Version'),
        'Id': data.get('Id'),
        'RecreateDirectories': data.get('RecreateDirectories')
    }

    targets = data.get('Targets')
    meta['Targets'] = parseTargets(targets, kape, quiet)

    if output == None:
        return meta
    out_path = Path("{}/{}".format(output, kape.parent))
    out_path.mkdir(parents=True, exist_ok=True)
    with open("{}/{}/{}.toml".format(output, kape.parent, kape.stem), 'wb') as file:
        tomli_w.dump(meta, file)

'''
Parse the Targets associated with the Kape file
Points to files to acquire or to other tkape files
If it points to another tkape file, we bundle them into one TOML file
'''
def parseTargets(targets, path, quiet):
    values = []
    for entry in targets:
        target_value = {}
        for key, value in entry.items():
            if '%user%' in str(value):
                value = value.replace('%user%', '*')
            target_value[key] = value
            if str(value).endswith(".tkape"):
                if not quiet:
                    print("Found compound target: {}. Category is '{}'. Going to bundle all the tkape files in '{}' into one TOML file".format(value, entry.get('Category'), value))

                find_taget = findKapeFile(Path(path.parts[0]), value)
                bundle = parseTKape(find_taget, quiet)
                values = values + bundle["Targets"]
        # If the FileMask is not set. Default is *
        # https://ericzimmerman.github.io/KapeDocs/#!Pages%5C2.1-Targets.md
        # Lets make that explicit
        if target_value.get('FileMask') == None:
            target_value['FileMask'] = "*"

        # If the Recursive is not set. Default is False
        # https://ericzimmerman.github.io/KapeDocs/#!Pages%5C2.1-Targets.md
        # Lets make that explicit
        if target_value.get('Recursive') == None:
            target_value['Recursive'] = False

        # If the AlwaysAddToQueue is not set. Default is False
        # https://ericzimmerman.github.io/KapeDocs/#!Pages%5C2.1-Targets.md
        # Lets make that explicit
        if target_value.get('AlwaysAddToQueue') == None:
            target_value['AlwaysAddToQueue'] = False
        
        # We do not add .tkape files to our toml output. Unless we want acquire a literal tkape file
        # The tkape targets below are not added
        '''
        Description: Evidence of execution related files
        Author: Eric Zimmerman
        Version: 1.1
        Id: 13ba1e33-4899-4843-adf0-c7e6a20d758a
        RecreateDirectories: true
        Targets:
            -
                Name: Amcache
                Category: ApplicationCompatibility
                Path: Amcache.tkape
            -
                Name: AppCompatPCA
                Category: ApplicationCompatibility
                Path: AppCompatPCA.tkape
            -
                Name: Prefetch
                Category: Prefetch
                Path: Prefetch.tkape
            -
                Name: RecentFileCache
                Category: ApplicationCompatibility
                Path: RecentFileCache.tkape
            -
                Name: Syscache
                Category: Syscache
                Path: Syscache.tkape

        # Documentation
        # ShimCache is not included in this Compound Target, as that would require pulling the entire SYSTEM Registry Hive. To ensure the ShimCache is pulled and parsed, use RegistryHivesSystem.tkape and parse with AppCompatCacheParser.mkape
        '''
        # Instead we bundle them all
        '''
        Description = "Evidence of execution related files"
        Author = "Eric Zimmerman"
        Version = 1.1
        Id = "13ba1e33-4899-4843-adf0-c7e6a20d758a"
        RecreateDirectories = true

        [[Targets]]
        Name = "Amcache"
        Category = "ApplicationCompatibility"
        Path = "C:\\Windows\\AppCompat\\Programs\\"
        FileMask = "Amcache.hve"
        Recursive = false
        AlwaysAddToQueue = false

        [[Targets]]
        Name = "Amcache transaction files"
        Category = "ApplicationCompatibility"
        Path = "C:\\Windows.old\\Windows\\AppCompat\\Programs\\"
        FileMask = "Amcache.hve.LOG*"
        Recursive = false
        AlwaysAddToQueue = false
        .....
        '''
        if ".tkape" in target_value.get("Path") and len(Path(target_value.get("Path")).parts) == 1:
            continue
        values.append(target_value)

    return values

'''
No AI was used to make this script XD
But probably it could have made it better XD
'''
def main():
    parser = argparse.ArgumentParser(
        prog="Kape2toml",
        description="Convert KAPE files to TOML format",
        epilog="Created for artemis: https://github.com/puffyCid/artemis"
    )

    parser.add_argument('-i', '--input', help="Path to the KapeFiles folder", required=True)
    parser.add_argument('-o', '--output', help="Path save the TOML files", required=True)
    parser.add_argument('-e', '--enable', help="Include the disabled KAPE files", action="store_true")
    parser.add_argument('-q', '--quiet', help="Suppress log messages", action="store_true")

    args = parser.parse_args()

    folder = Path(args.input)
    output = Path(args.output)
    enable = args.enable
    quiet = args.quiet

    print("Scanning folder at {}".format(folder))

    recurseFolder(folder, output, enable, quiet)
    print("KAPE files converted to TOML files at '{}'".format(output))

if __name__ == "__main__":
    main()