import csv

def main():
    with open('properties.csv', mode='r') as file:
        csvFile = csv.reader(file)
        for lines in csvFile:
           if lines[1] == 'n/a' or lines[0].lower().startswith('pidlid'):
               continue
           # print("{},".format(lines[0]))
           # print('"{}_{}" => names.push(PropertyName::{}),'.format(lines[1].lower(), lines[3].lower(), lines[0]))
           # print('"{}_{}"'.format(lines[1].lower(), lines[3].lower()))
        file.seek(0)
        for lines in csvFile:
            if lines[1] == 'n/a' or not lines[0].lower().startswith('pidlid'):
                continue
            # print('"{}" => ids.push(PropertyId::{}),'.format(lines[1].lower(), lines[0]))
            # print("{},".format(lines[1]))

if __name__ == "__main__":
    main()