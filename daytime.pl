#!/usr/bin/perl

use strict;

my %data = ('s' => 0, 'm' => 0, 't' => 0, 'w' => 0, 'r' => 0, 'f' => 0, 'a' => 0);

while(<>){
    #if(/\[.\].*\[(.*)\|.*\|(\d+(\.\d*)?|)\|(s:x+|m:x+|t:x+|w:x+|r:x+|f:x+|a:x+)\]/){
    #if(/\[.\].*\[(.*)\|.*\|(\d+(\.\d*)?|)\|(s:x+)*,*(m:x+)*,*(t:x+)*,*(w:x+)*,*(r:x+)*,*(f:x+)*,*(a:x+)*\]/){
    if(/\[.\].*\[(.*)\|.*\|(\d+(\.\d*)?|)\|([smtwrfa]+.*)\]/){
        my @pomos = split(',',$4);
        foreach (@pomos)
        {
            my ($day, $pomos) = split(':', $_);
            $data{$day} += length($pomos);
        }
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
