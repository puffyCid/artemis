// https://www.mandiant.com/sites/default/files/2021-09/wp-windows-management-instrumentation.pdf

/*
 * very complex O.o
 *
 * TODO:
 * 2. parse class definitions
 * 2.1 parse qualifiers
 * 2.2 parser property descriptors
 * 3. tests
 */

/*
* default - 892f8db69c4edfbc68165c91087b7a08323f6ce5b5ef342c0f93e02a0590bfc4
* subscription - e1dd43413ed9fd9c458d2051f082d1d739399b29035b455f09073926e5ed9870
* root - e8c4f9926e52e9240c37c4e59745ceb61a67a77c9f6692ea4295a97e0af583c5

* Steps to parse wmi (in progress)
* 0. Lookup namespaces: ROOT\default and ROOT\subscription should get NS_<SHA256>
* 0.1 Find classes __EventFilter, __EventConsumer, __FilterToConsumerBinding
* 0.2. Figure out how logical_page.record_id.size works. Do u need to use mappings.data files????
* 1. Parse index.btr
* 2. take the page_number.record_id.data_size and lookup the info in the objects.data file
* 3. ?? get class info??




0. Parse namespace NS_FCBAF5A1255D45B1176570C0B63AA60199749700C79A11D5811D54A83A1F4EFD)
  a. Parse __Namespace CD_64659AB9F8F1C4B568DB6438BAE11B26EE8F93CB5F8195E21E8C383D6C44CC41 and get all the namespaces
  b. create vec of sha256 NS_<namespace>
  c. Will need to parse the _EventConsumer, _EventFilter, _FilterToEventConsumer classes for all namespaces to check for WMI persistance
  d. \\.\ROOT\default and \\.\ROOT\subscription r most common

Consider the WQL query 'SELECT Description FROM \\.\ROOT\default\ExistingClass WHERE Name=“SomeName”'
that fetches the property named Description from an instance of the ExistingClass class named SomeName.

The WMI service performs the following operations via the CIM repository to resolve the data:

1. Locate the \\.\ROOT\default namespace
  a. Build the index key
  b. Ensure namespace exists via index key lookup
2. Find the class definition for 'ExistingClass'
  a. Build the index key
  b. Do index key lookup to get object location
  c. Get object data from objects.data

3. Enumerate class definitions of the ancestors of ExistingClass
  a. Parse object definition header
  b. Recursively lookup class definitions of parent classes (steps 1-3)
4. Build the class layout from the class definitions
5. Find the class instance object of 'ExistingClass' with Name equal to 'SomeName'
  a. Build the index key
  b. Do index key lookup to get object location
  c. Get object data from objects.data

6. Parse the class instance object using the class layout
7. Return the value from property 'Description'


* Make it detects all entries made by https://github.com/mgreen27/mgreen27.github.io/blob/master/static/other/WMIEventingNoisemaker/WmiEventingNoisemaker.ps1
* Need to be able to parse/scan all namespaces (unknown hashes)
*/
