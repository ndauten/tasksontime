#!/usr/bin/perl

use strict;

my %data = ('s' => 0, 'm' => 0, 't' => 0, 'w' => 0, 'r' => 0, 'f' => 0, 'a' => 0);

while(<>){
    if(/\[.\].*\[(.*)\|.*\|(\d+(\.\d*)?|)\|(s:(x+)|m:(x+)|t:(x+)|w:(x+)|r:(x+)|f:(x+)|a:(x+))\]/){
        my ($day, $pomos) = split(':', $4);
        #print $_, "- Day:", $day, ", pomos: ", $pomos, "\n";
        $data{$day} += length($pomos);
    }
}
print "s:",$data{'s'};
print " m:",$data{'m'};
print " t:",$data{'t'};
print " w:",$data{'w'};
print " t:",$data{'r'};
print " f:",$data{'f'};
print " s:",$data{'a'};
print "\n";
