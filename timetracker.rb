#!/usr/bin/ruby
#

require 'yaml'

class BrickTimeRecord
    attr_writer :startTime, :startTime, :name
    @startTime = 0
    @endTime = 0
    @name = ""
    def initialize(stime, etime, tags)
        @startTime = stime
        @endTime = etime
        @tags = Array.new
        @tags = tags
    end
    def to_s
        print "Name: #{@name} --- Start Time: #{@startTime} --- End Time: #{@endTime}"
    end
end

class Brick
    attr_reader :children, :parent, :name
    def initialize(name, parent, tags)
        @name = name
        @parent = parent
        @tags = Array.new           # This is an array of tags for this brick
        @tags.push(tags)
        @timeWorked = Array.new     # This is an array of TimeEntry objects
        @children = Hash.new       # Names of children -- index is a string
    end
    def addChild(child)
        @children[child.name] = child
    end
    def recordTime(te)
        @timeWorked.push(te)        
    end
    def to_s
        print "Brick Name: #{@name}, Parent: #{@parent}, Children: #{@children}, Tags: #{@tags}"
    end
end

class BrickTree
    def initialize()
        @root = Brick.new("root", "", ["all"])
        @tree = Hash.new
        @tree["root"] = {
            'brick' => "root", 
            'parent' => '', 
            'tags' => ["all"], 
            'timeWorked' => [], 
            'children' => [] 
        }
    end

    def addBrick(bname, parentName, tags)
        # Check that the parent node is in the tree
        raise "Parent node: #{bname}  does not exist." unless @tree.has_key?(parentName)

        # Now add the data to the objects of the parent and create the child
        #parentBrick = getBrick(parentName, @root) # @tree[parentName]["brick"]
        #newBrick = Brick.new(bname, parentBrick, tags)
        #parentBrick.addChild(newBrick)

        # Update @tree indexing structure -- playing around a bit here with the
        # hash as a full representation of the tree and data.
        @tree[parentName]['children'].push(bname)
        @tree[bname] = {
            'brick' => bname, 
            'parent' => parentName, 
            'tags' => tags, 
            'timeWorked' => [], 
            'children' => [] 
        }
    end

    def getBrickFromName(bname)
        getBrick(bname, @root)
    end 

    def getBrick(bname, bnode)
        #puts "Checking bnode: #{bnode.name} for match..."
        return bnode if (bname == bnode.name)

        bnode.children.each {|key, b|
            return getBrick(bname, b)
        }

        return false
    end

    # This function adds a brick time instance to the brick of the given name
    def recordTime(bname, tstart, tend, tags)
        # Check that the parent node is in the tree
        raise "Brick node: #{bname} does not exist." unless @tree.has_key?(bname)
        getBrick(bname, @root).recordTime(BrickTimeRecord.new(tstart, tend, tags))
        @tree[bname]['timeWorked'].push(BrickTimeRecord.new(tstart, tend, tags))
    end

    def isBrick(b)
        @tree.has_key?(b)
    end

    def hasChild(parent,child)
        # TODO: Need an exception here for parent does not exist 
        raise 'Parent node does not exist.' unless @tree.has_key?(parent)
        @tree[parent]['children'].include?(child)
    end
    
    def printTree(name, level)
        print "  --+"*(level), "#{name}:\n"
        @tree[name]['children'].each { |brickName|
            printTree(brickName, level+1)
        }
    end

    def to_s
        printTree("root", 1)
    end
end

def printTimeDiff(start)
    finish = Time.now()
    seconds = finish - start
    print "\n\t**** The time difference: "
    print "%02d:%02d:%02d ****\n" % [
        seconds / (60*60),
        seconds / 60 % 60,
        seconds % 60
    ]
    print "\nThe finish time: #{finish}\n"
end
        

#print "\tCommand [r]: "
#while(gets.chomp() =~ /rh/) do
#end
#puts "Starting..."
#begin
    #start = Time.now()
    #puts start
    #print "\tCommand [r]: "
    #while(gets.chomp() =~ /r/) do
        #printTimeDiff(start)
        #puts "\nRestarting..."
        #start = Time.now()
        #puts start
    #end
    #printTimeDiff(start)
#rescue Interrupt => e
    #print "\n"
    #printTimeDiff(start)
    #exit
#end
