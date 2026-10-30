//! Tests for the half of the sync engine that writes to the disk: the code
//! that applies a removal coming from Studio and deletes files with it.
//!
//! Everything runs against the in-memory VFS, so no test touches the disk.

mod vfs_memory {
	use polaris::vfs::Vfs;
	use std::path::Path;

	#[test]
	fn create_dir_creates_every_parent() {
		let vfs = Vfs::new_virtual();

		vfs.create_dir(Path::new("project/src/Foo")).unwrap();

		assert!(vfs.is_dir(Path::new("project")));
		assert!(vfs.is_dir(Path::new("project/src")));
		assert!(vfs.is_dir(Path::new("project/src/Foo")));
		assert!(!vfs.exists(Path::new("project/src/Bar")));
	}

	#[test]
	fn remove_takes_the_whole_subtree() {
		let vfs = Vfs::new_virtual();

		vfs.create_dir(Path::new("project/src/Foo")).unwrap();
		vfs.write(Path::new("project/src/Foo/init.luau"), b"return 1").unwrap();
		vfs.write(Path::new("project/src/Foo/Nested/init.luau"), b"return 2")
			.unwrap();
		vfs.write(Path::new("project/src/Bar.luau"), b"return 3").unwrap();

		vfs.remove(Path::new("project/src/Foo")).unwrap();

		assert!(!vfs.exists(Path::new("project/src/Foo")));
		assert!(!vfs.exists(Path::new("project/src/Foo/init.luau")));
		assert!(!vfs.exists(Path::new("project/src/Foo/Nested/init.luau")));
		assert!(vfs.exists(Path::new("project/src/Bar.luau")));
	}

	#[test]
	fn remove_missing_path_is_an_error() {
		let vfs = Vfs::new_virtual();

		assert!(vfs.remove(Path::new("project/nope")).is_err());
	}
}

mod removal {
	use polaris::{
		core::{meta::Meta, meta::Source, processor::write, snapshot::Snapshot, tree::Tree},
		vfs::Vfs,
	};
	use rbx_dom_weak::{types::Ref, InstanceBuilder, WeakDom};
	use std::path::Path;

	fn empty_tree() -> Tree {
		Tree::new(Snapshot::new().with_name("DataModel"))
	}

	fn insert(tree: &mut Tree, name: &str, source: Source) -> Ref {
		let root = tree.root_ref();

		let snapshot = Snapshot::new()
			.with_name(name)
			.with_meta(Meta::new().with_source(source));

		tree.insert_instance(snapshot, root)
	}

	// A Ref that belongs to another dom, so it is guaranteed to be missing
	fn unused_ref() -> Ref {
		let mut dom = WeakDom::new(InstanceBuilder::new("Folder"));
		let root = dom.root_ref();

		dom.insert(root, InstanceBuilder::new("Folder"))
	}

	#[test]
	fn removing_an_instance_deletes_its_folder_and_nothing_else() {
		let mut tree = empty_tree();
		let vfs = Vfs::new_virtual();

		vfs.create_dir(Path::new("project/src/Foo")).unwrap();
		vfs.write(Path::new("project/src/Foo/init.luau"), b"return 1").unwrap();
		vfs.write(Path::new("project/src/Foo/Child.luau"), b"return 2").unwrap();
		vfs.write(Path::new("project/src/Bar.luau"), b"return 3").unwrap();

		let neighbour = insert(&mut tree, "Bar", Source::file(Path::new("project/src/Bar.luau")));
		let id = insert(&mut tree, "Foo", Source::directory(Path::new("project/src/Foo")));

		write::apply_removal(id, &mut tree, &vfs).unwrap();

		assert!(!vfs.exists(Path::new("project/src/Foo")));
		assert!(!vfs.exists(Path::new("project/src/Foo/init.luau")));
		assert!(!vfs.exists(Path::new("project/src/Foo/Child.luau")));
		assert!(vfs.exists(Path::new("project/src/Bar.luau")));

		assert!(!tree.exists(id));
		assert!(tree.exists(neighbour));
	}

	#[test]
	fn removing_a_script_deletes_only_its_file() {
		let mut tree = empty_tree();
		let vfs = Vfs::new_virtual();

		vfs.write(Path::new("project/src/Foo.luau"), b"return 1").unwrap();
		vfs.write(Path::new("project/src/Bar.luau"), b"return 2").unwrap();

		let neighbour = insert(&mut tree, "Bar", Source::file(Path::new("project/src/Bar.luau")));
		let id = insert(&mut tree, "Foo", Source::file(Path::new("project/src/Foo.luau")));

		write::apply_removal(id, &mut tree, &vfs).unwrap();

		assert!(!vfs.exists(Path::new("project/src/Foo.luau")));
		assert!(vfs.exists(Path::new("project/src/Bar.luau")));
		assert!(tree.exists(neighbour));
	}

	#[test]
	fn removing_an_instance_with_children_deletes_its_folder_and_file() {
		let mut tree = empty_tree();
		let vfs = Vfs::new_virtual();

		vfs.create_dir(Path::new("project/src/Foo")).unwrap();
		vfs.write(Path::new("project/src/Foo/init.luau"), b"return 1").unwrap();
		vfs.write(Path::new("project/src/Foo/init.meta.json"), b"{}").unwrap();

		let _neighbour = insert(&mut tree, "Bar", Source::file(Path::new("project/src/Bar.luau")));
		let id = insert(
			&mut tree,
			"Foo",
			Source::child_file(Path::new("project/src/Foo"), Path::new("project/src/Foo/init.luau")),
		);

		write::apply_removal(id, &mut tree, &vfs).unwrap();

		assert!(!vfs.exists(Path::new("project/src/Foo")));
		assert!(!vfs.exists(Path::new("project/src/Foo/init.luau")));
		assert!(!vfs.exists(Path::new("project/src/Foo/init.meta.json")));
	}

	#[test]
	fn a_sibling_whose_name_starts_the_same_survives() {
		let mut tree = empty_tree();
		let vfs = Vfs::new_virtual();

		vfs.create_dir(Path::new("project/src/Foo")).unwrap();
		vfs.write(Path::new("project/src/Foo/init.luau"), b"return 1").unwrap();
		vfs.create_dir(Path::new("project/src/Foo2")).unwrap();
		vfs.write(Path::new("project/src/Foo2/init.luau"), b"return 2").unwrap();

		let neighbour = insert(&mut tree, "Foo2", Source::directory(Path::new("project/src/Foo2")));
		let id = insert(&mut tree, "Foo", Source::directory(Path::new("project/src/Foo")));

		write::apply_removal(id, &mut tree, &vfs).unwrap();

		assert!(!vfs.exists(Path::new("project/src/Foo")));
		assert!(vfs.exists(Path::new("project/src/Foo2")));
		assert!(vfs.exists(Path::new("project/src/Foo2/init.luau")));
		assert!(tree.exists(neighbour));
	}

	#[test]
	fn removing_an_unknown_instance_does_nothing() {
		let mut tree = empty_tree();
		let vfs = Vfs::new_virtual();

		vfs.create_dir(Path::new("project/src/Foo")).unwrap();

		let neighbour = insert(&mut tree, "Foo", Source::directory(Path::new("project/src/Foo")));

		write::apply_removal(unused_ref(), &mut tree, &vfs).unwrap();

		assert!(vfs.exists(Path::new("project/src/Foo")));
		assert!(tree.exists(neighbour));
	}

	#[test]
	fn removing_the_only_child_keeps_the_rest_of_the_folder() {
		let mut tree = empty_tree();
		let vfs = Vfs::new_virtual();

		vfs.create_dir(Path::new("project/src/Foo")).unwrap();
		vfs.write(Path::new("project/src/Foo/init.luau"), b"return 1").unwrap();
		vfs.write(Path::new("project/src/Other.luau"), b"return 2").unwrap();

		let id = insert(&mut tree, "Foo", Source::directory(Path::new("project/src/Foo")));

		write::apply_removal(id, &mut tree, &vfs).unwrap();

		assert!(!vfs.exists(Path::new("project/src/Foo")));
		assert!(vfs.exists(Path::new("project/src/Other.luau")));
	}
}

mod changes {
	use polaris::core::{
		changes::Changes,
		snapshot::{Snapshot, UpdatedSnapshot},
	};
	use rbx_dom_weak::types::Ref;

	#[test]
	fn empty_batch_reports_nothing() {
		let changes = Changes::new();

		assert!(changes.is_empty());
		assert_eq!(changes.total(), 0);
	}

	#[test]
	fn each_kind_counts_once() {
		let mut changes = Changes::new();

		changes.add(Snapshot::new(), Ref::none());
		changes.update(UpdatedSnapshot::new(Ref::none()));
		changes.remove(Ref::none());

		assert_eq!(changes.additions.len(), 1);
		assert_eq!(changes.updates.len(), 1);
		assert_eq!(changes.removals.len(), 1);
		assert_eq!(changes.total(), 3);
		assert!(!changes.is_empty());
	}

	#[test]
	fn extend_sums_both_batches() {
		let mut changes = Changes::new();
		changes.remove(Ref::none());

		let mut other = Changes::new();
		other.update(UpdatedSnapshot::new(Ref::none()));
		other.remove(Ref::none());

		changes.extend(other);

		assert_eq!(changes.total(), 3);
		assert_eq!(changes.updates.len(), 1);
		assert_eq!(changes.removals.len(), 2);
	}
}
