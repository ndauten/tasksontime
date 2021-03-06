#!/usr/bin/ruby
#

#require 'test_helper'
require 'test/unit'
require '../lib/timetracker/brick_tree.rb'
require '../lib/timetracker/timer.rb'
require 'yaml'

class TestBrickTree < Test::Unit::TestCase
    # Note: setup and teardown occur before and after each test invocation
    def setup
        @test_file_name = "test_bricks.yaml"
        @bt = BrickTree.new
        @bt.addBrick("c1", "root", ["admin","emai"])
        assert(@bt.isBrick("c1"))
        assert(@bt.hasChild("root","c1"))

        @bt.addBrick("s2", "root", [])
        assert(@bt.isBrick("s2"))
        assert(@bt.hasChild("root","s2"))

        @bt.addBrick("c11", "c1", ["emai"])
        assert(@bt.isBrick("c11"))
        assert(@bt.hasChild("c1","c11"))

        @bt.addBrick("c111", "c11", [])
        assert(@bt.isBrick("c111"))
        assert(@bt.hasChild("c11","c111"))

        now = Time.now
        @bt.recordTime("Test task.", "c11", now - 10, now, [])
        now = Time.now
        @bt.recordTime("Test task.", "c11", now - 10, now, [])
        
        File.open(@test_file_name, 'w') {|f| f << @bt.to_yaml }
    end

    def teardown
    end

    def test_save_and_read_yaml
        assert_equal(YAML.load_file(@test_file_name).to_yaml,  @bt.to_yaml)
    end

    def test_save_and_modify_yaml
        newtree = YAML.load_file(@test_file_name)

        newtree.addBrick("c3", "root", ["emai"])
        assert(newtree.isBrick("c3"))
        assert(newtree.hasChild("root","c3"))

        newtree.addBrick("c4", "s2", ["waste-of-time"])
        assert(newtree.isBrick("c4"))
        assert(newtree.hasChild("s2","c4"))

        now = Time.now
        newtree.recordTime("Test task1", "c3", now - 10, now, [])
    end

    def test_brick_add_time
        puts @bt.to_yaml
    end

    def test_brick_move_with_children
        tree = BrickTree.new    

        # Create a node three deep on c1 branch
        tree.addBrick("c1","root",[])
        tree.addBrick("c11","c1",[])
        tree.addBrick("c111","c11",[])

        # Create some other top level branches
        tree.addBrick("c2","root",[])
        tree.addBrick("c3","root",[])
        tree.addBrick("c4","root",[])
        tree.prettyPrintFullTree()
        
        # Verify c1 has c11 prior to moving
        assert(tree.hasChild("c1","c11"))
        assert(tree.hasChild("root","c1"))

        # Execute the move with children
        tree.moveWithChildren("c11","c4")

        # Do some checking: c11 should not be a child of c2 and not c1
        tree.prettyPrintFullTree()
        assert(tree.hasChild("c4","c11"))
        assert(!tree.hasChild("c1","c11"))

        # c11 should have c111 as its child still
        assert(tree.hasChild("c11","c111"))

        # veryify the new parent of c11
        assert_equal(tree.getParent("c11")['brick'], "c4")
    end

    def test_brick_post_order
        @bt.traverse_postorder("root") {|brick|
            puts brick['brick'] 
        }
    end

    def test_brick_week_time
        h = Hash.new(0)
        @bt.traverse_postorder do |brick|
            puts @bt.brickTotalTimeDirect(brick['brick'])
            h[brick['brick']] = @bt.brickTotalTimeDirect(brick['brick'])
        end
        puts h
    end

    def test_print_btree
        #@bt.prettyPrint("root", 0)
    end
    
    def test_print_yaml
        puts @bt.to_yaml
    end
end

class TestBrick < Test::Unit::TestCase
    def setup
    end
    def teardown
    end
    def test_add_brick
    end
    def test_print_tree
    end
end
