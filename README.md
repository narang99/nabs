# nabs: not a build system

The tool only does diffing. Only for simple projects, small projects, its fine  
Not for huge monorepos where a ridiculous number of developers work  
It is only needed for figuring out what to run given some changeset  

As such, it does nothing like running any build steps. All we do is provide diffing  

Start with just python, find requirements.txt and parse it, this gives us

Targets right now are simply file paths  
Given a file path, who all are affected is the simple question  

We need to find all files. Every package does need a boundary (nabs.json).  

given a nabs.json, we have a simple connected graph.  just walk that graph for querying  
the monorepo root is defined by workspace.json  

A target is defined by using the good old bazel terminology: //packages/python/qsync_stream is an example target  


- First create the graph after walking the file system  
  - Create a graph, each node is a nabs package  
  - we somehow create edges in the graph
  - Given a node, we want to simply find all downstream children
  - We can go one more way and provide the most optimal way of running CI (that is, people wait only for whatever is running)
    - this would be done later though



