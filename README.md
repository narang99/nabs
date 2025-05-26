# nabs: Not a Build System

**NOTE:** This project is not in a very useful state right now, I'm only planning to use it personal projects to see if its any good.  

`nabs` models your monorepo as a graph and finds all the affected packages for a given changeset.  

When using a monorepo, it is very useful to only selectively run CI pipelines. A common workflow is this:
- find the diff in the PR using git
- find all the affected packages and services using this diff
- run scripts/pipelines for all the affected packages

This can be done in `bazel` and friends, for example, using `bazel query 'rdeps(//my-target)'`. All the build systems with monorepos give this feature.   
These build tools come with their own set of pains though:
- steep learning curve
- high maintenance cost
- bad IDE integration
- huge upfront investment in building a custom CI

Tools like `bazel` are extremely feature-rich, fast and provide some amazing qualities (like hermeticity). They are however, built for scale, where you have a dedicated engineering team taking care of it. They give the fast monorepo experience at scale.  
`nabs` is not meant for those use-cases. `nabs` is for mid-sized engineering teams, who just want a simple monorepo setup while using existing tooling and infrastructure.  

The main benefit I saw from a monorepo is that we could change all the code in one PR. Raising multiple PRs is a big velocity killer.  
I generally prefer all code to be in the same repo. This works well for sometime, until your tests and pipelines start taking a long time. Or you now have multiple applications/services in the same monorepo (whose deployment should be done independently)   

This is where `nabs` can come in. The only explicit goal of `nabs` is to track dependencies, it is neither a build executor, nor a test runner, nor a remote execution framework. `nabs` does not care about purity, it does not force any ideology on you.  
An explicit goal for `nabs` is to work with current tooling. In your monorepo, you could add an empty `nabs.json` file and `nabs` would start tracking it as a package. If you have `requirements.txt` in that package, `nabs` would automatically start treating it as a python package, find all the local dependencies in your workspace and start tracking them recursively.  
Thats it, this makes it extremely trivial to add new build systems and languages.  

# Getting started

Add an empty `workspace.json` in the root of your monorepo.  
```sh
echo "{}" > workspace.json
```

In every package in the monorepo, add an empty `nabs.json` file
```sh
echo "{}" > nabs.json
```
`nabs` would start tracking your package. Do this for every package in the monorepo.  


You can take a look at the graph `nabs` creates using

```sh
# from any folder inside the workspace
nabs graph
```

`nabs changeset` takes a list of files as input, finds the packages those files belong to, and finds all the packages which transitively depend on these packages. The idea is that you can run the CI pipelines for these specific packages.  

The usual workflow for selectively running scripts could be
```sh
CHANGED_GIT_FILES=$(git diff-tree --no-commit-id --name-only -r origin/main my-awesome-branch)
AFFECTED_PACKAGES=$(echo $CHANGED_GIT_FILES | nabs changeset)

# run a script inside the package directory
# or you could run a github action, or a jenkins pipeline
echo $AFFECTED_PACKAGES | while read pkg_dir; do $pkg_dir/run_test.sh; done
```

## Supported build systems/languages
- Python
  - requirements.txt
- Rust 
  - Cargo.toml


## should I use `nabs`?

- `nabs` is supposed to be useful in that spot where your monorepo has started taking a lot of your CI time but you don't want to do a big investment in a fancy build tool.    
- you should not use `nabs` if you have huge repositories with many developers, in this case, `bazel` and other build tools are clear winners.  