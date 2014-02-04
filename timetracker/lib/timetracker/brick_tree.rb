#!/usr/bin/ruby
#

class BrickTimeRecord
    attr_writer :startTime, :startTime, :name
    @startTime = 0
    @endTime = 0
    @name = ""

    def initialize(tname="", stime=0, etime=0, tags=[])
        @startTime = stime
        @endTime = etime
        @tags = Array.new
        @tags = tags
        @name = tname
    end

    def duration  #Note this is in seconds
        @endTime - @startTime
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

#
# ----------------------------------------------------------------------------
# Description: 
#   The BrickTree represents a tree of roles/categories/tasks/projects/bricks
#   where each node is something that time can be assigned to. In this way the
#   brick can have encoded by the tree all time associated with it by
#   accumulating the time worked in each of its children nodes. 
# ----------------------------------------------------------------------------
#
class BrickTree
    def initialize()
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
        raise "Parent node: #{parentName} does not exist." unless @tree.has_key?(parentName)

        # Make sure we have a new bname 
        raise "Empty brick name, will not add!" if (bname.nil?)

        # Check to see if the brick already exists, if so raise an exception
        raise "Brick node: #{bname} already exists in the tree." if @tree.has_key?(bname)

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

    #
    #-- 
    # Method: removeSubTree
    #
    # Description:
    #   This method will do a preorder traversal of the tree and remove all of
    #   a particular subtree starting at the given brick. Once all of the tree
    #   is removed it must then remove the link in the parent's children list. 
    #
    # Input:
    #   - Name of the brick to eliminate
    #--  
    #
    def removeSubTree(bname)
        # Remove the brick from it's parent
        # Recursively remove this brick and its children 
        self.traverse_postorder(bname) do |brick|
            # Remove the brick from the hash
            self.removeBrick(brick['brick'])

            # Now remove the parent node's link to the brick
            self.removeChildLink(brick['parent'], brick['brick'])
        end
    end

    def removeBrick(bname)
        @tree.delete(bname)
    end

    def removeChildLink(parent, child)
        @tree[parent]['children'].delete(child)
    end

    def getParent(bname)
        @tree[@tree[bname]["parent"]]
    end

    def getBrickFromName(bname)
        getBrick(bname, @root)
    end 

    # This function is deprecated and might need to be removed. An earlier data
    # structure for the brick was to use a class with pointers, such as in a
    # traditional C type data structure. Now we just use the hash.
    def getBrick(bname, bnode)
        #puts "Checking bnode: #{bnode.name} for match..."
        return bnode if (bname == bnode.name)

        bnode.children.each {|key, b|
            return getBrick(bname, b)
        }

        return false
    end

    # This function adds a brick time instance to the brick of the given name
    def recordTime(tname, bname, tstart, tend, tags)
        # Check that the parent node is in the tree
        raise "Brick node: #{bname} does not exist." unless @tree.has_key?(bname)

        # Create a time record and add it to the timeWorked array
        @tree[bname]['timeWorked'].push(BrickTimeRecord.new(tname, tstart, tend, tags))
    end

    def brickTotalTimeDirect(bname)
        # Check that the parent node is in the tree
        raise "<bttd> Brick node: #{bname} does not exist." unless @tree.has_key?(bname)
        sum = 0
        @tree[bname]['timeWorked'].each do |btr|
            sum = sum + btr.duration
        end
        sum
    end

    def brickTotalTimeAggregate(brickName)
        raise "Brick node: #{brickName} does not exist can't calculate time." unless @tree.has_key?(brickName)
        brickAggregateTime = 0
        self.traverse_postorder(brickName) do |brick|
            brickAggregateTime += brickTotalTimeDirect(brick['brick'])
        end
        brickAggregateTime
    end
    
    # This function includes a yield, which is a closure passed from the
    # context of the caller. It also does a depth first, visit children then
    # parent traversal order.
    def traverse_postorder(brick="root", &lambdaF)
        raise "Brick node: #{brick} does not exist can't traverse." unless @tree.has_key?(brick)

        @tree[brick]['children'].each {|bc| traverse_postorder(bc, &lambdaF) }

        # We pass the brick into the closure associated with the function call
        yield(@tree[brick])
    end

    # 
    # ---
    # Function: traverse_preorder
    #
    # Description:
    #   Traverse the tree starting at brick in preorder, which means to visit
    #   the parent then the children.
    #
    # Input:
    #   brick: The node to start the traversal at
    #   &lambdaF: The lambda function to execute for visiting the node
    #---
    #
    def traverse_preorder(brickName="root", &labmdaF)
        raise "Brick node: #{brickName} does not exist can't traverse." unless @tree.has_key?(brickName)
        
        # Visit the parent first
        yield(@tree[brickName])

        # Then recurse into the children
        @tree[brickName]['children'].each {|bc| traverse_preorder(bc, &lambdaF) }
    end


    def isBrick(b)
        @tree.has_key?(b)
    end

    def hasChild(parent,child)
        # TODO: Need an exception here for parent does not exist 
        raise 'Parent node does not exist.' unless @tree.has_key?(parent)
        @tree[parent]['children'].include?(child)
    end
    
    # attempts to pretty print the tree.
    def prettyPrint(name, level)
        brickTimePretty = TimeUtils.timeDiffPretty(brickTotalTimeAggregate(name))
        #print "   "*(level), '- ', "#{name} [#{brickTimePretty}]: ", "."*5," #{brickTimePretty}\n"
        print "    "*(level), '- ', "#{name} [#{brickTimePretty}] \n"
        @tree[name]['children'].each { |brickName|
            raise "Pretty Print: child node #{brickName} is not in the tree." unless @tree.has_key?(brickName)
            prettyPrint(brickName, level+1)
        }
    end

    def prettyPrintFullTree
        prettyPrint("root", 0)
    end

    def printSubtree(brickName="root", indent=0)
        traverse_preorder(brickName) do |brick|
            print brick['brick'], "\n"
            #print ' ' * indent, "#{brick['brick']}: #{brickTotalTimeAggregate(brick['brick'])} \n"
            #indent += 1
        end
    end

    def to_s(indent=0)
        printSubtree("root", 0)
    end
end
