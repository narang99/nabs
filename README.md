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



- rdeps done

# creating the graph

- nabs would first read the whole file tree and try to detect packages
- right now, any directory containing nabs.json is a package
- read the file and try to infer what build system is being used
- inference can return multiple detected build systems, in this case, nabs would error out
  - users are required to fix the build system in nabs.json
  - you could technically have multiple build systems in the same directory, and nabs should be able to work hmmm
  - the most generic case is that someone has x build systems in the directory
  - nabs should detect and create a node for each of those



- given a package, the inferrer returns
  - current Target definition
  - parent Target defs

- a single inferrer can return multiple targets
- multiple inferrers can return single targets
- an inferrer can break early
- an inferrer can give no target

- nabs.json inferrer: returns multiple targets and requires shortcircuting
- running cargo inferrer on poetry gives None
- a project containing both cargo.toml and requirements.txt returns 2 targets (1 target per inferrer)
- We want to differentiate between allowed multiple targets and unintended multiple targets

- Given a target def, we want to verify if its valid, get the inferred result of that target
  - if the given target def is not in the inferred targets, this is not a valid dependency of the target
  - for now we panic?


- one complication is path handling, i need to add methods for converting paths to target names and vice-versa
  - this seems the most logical way to do this