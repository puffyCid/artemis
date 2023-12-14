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
*/
