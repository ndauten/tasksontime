#!/usr/bin/perl -w
use strict;
use DateTime;
use POSIX qw(strftime);
use Getopt::Long;

my ($points, $pomos, $plan) = (0,0,0);
my ($tpoints, $tpomos, $tplan, $texp) = (0,0,0);
my %tags;
my %tagsplanned;
my @days = qw/Sunday Monday Tuesday Wednesday Thursday Friday Saturday/;

sub backup(){
    print "Backing up to stats file: $_[0]\n\n";
    system("cp ./stats.txt stats.archive");
    #open STATS,">>",$_[0] or die "\n\n!!! non existent file !!! $1\n\n";
}

#### OPTIONS ####
my $today = DateTime->now()->subtract( hours => 6);
my $date = ''; 
my $file = 'today.txt';
my $help = '';
my $summary = '';
my $write = '';
my $backup = '';
my $itemize = '';

GetOptions ('summary' => \$summary, 
            'itemize' => \$itemize,
            'help' => \$help, 
            'file=s' => \$file, 
            'day=s' => \$today,
            'date=s' => \$date,
            'write' => \$write,
            'backup' => \$backup
);

# Itemize includes a summary by default
if($itemize){$summary="summary"}

# If we summarize then don't also itemize else we print day with summary
if($summary){
    $today="all";
} else {
    $itemize="itemize";
}

if($help){
    print "\n";
    print "Usage: ./pomo_point_count.pl -f todo_file (default: today.txt)";
    print "\n\n";
    exit;
}elsif($backup){
    &backup("./stats.txt");
}

### Time Init and Print Out Week to Screen ###
my $now = localtime;
my $dayofweek = DateTime->now()->dow;
my $dt = '';
my $dtf = '';
$dt = DateTime->now()->subtract( hours => 6);
$dt->subtract( days => $dayofweek );
if($date){
    my @dateArr = split('\.',"$date");
    $dt->set(year=>$dateArr[2]);
    $dt->set(month=>$dateArr[0]);
    $dt->set(day=>$dateArr[1]);
}
$dtf = $dt->clone();
$dtf->add( days=>6 );
my $l1 = sprintf "|  Sunday %s.%s.%s ---> Saturday %s.%s.%s  |",
    $dt->month,$dt->day,$dt->year,$dtf->month,$dtf->day,$dtf->year;
my $duration = sprintf "%57s",$l1;
my $len = length($l1);
my $line = sprintf "\n%57s\n","+" . ('-' x ($len-2)) . "+";
print $line;
print $duration;
print $line;

# Function that parses the file, collects stats for the given day, and prints 
# format: [complete=X,unfinished=' '] Task [project|points|exppomos|pomos]
sub print_day_stats(){
    if( ! open TODOFH, $file){die "\n\n!!! non existent file !!!\n\n"};
    my ($exp,$eff,$flag);
    my $today = $_[0];
    while(<TODOFH>){
        # Match for total pomos expected for efficiency
        if(/===== Expected Weekly Pomos: ([\d]+)/) { $texp = $1; }
        # Match for counting pomos
        if(/===== (.*) \[[\d]*:([\d]+).*/){
            if($1 eq $today){
                $flag = 1;
                $exp=$2;
            }elsif($flag){
                if($exp > 0){
                    $eff = $pomos/$exp;
                }else{
                    $eff = "0.00"
                }
                printf "\t%2s %15d %16s\t%11.02f\n",$points,$plan,$pomos,$eff;
                $tplan += $plan; $tpoints += $points, $tpomos += $pomos; 
                #$texp += $exp;
                $plan = 0; $points = 0; $pomos = 0; 
                close TODOFH;
                return;
            }
        }
        if($flag){
            if(/\|(x+)\]/){
                $pomos+=length($1);
            }
            # Match for counting points
            if(/\[x\].*\[.*\|([[\d]+|\.[\d]+|\d\.[\d]+])\|.*\|.*\]/){ 
                $points+=int($1); 
            }
            # Match for counting planned pomos
            if(/\[.*\|.*\|([\d]+[\.[\d]*|""])\|.*\]/){
                $plan+=int($1);
            }
            if(/\[.\].*\[(.*)\|.*\|([\d]+[\.[\d]*|""])\|(x*)\]/){
                my $newct;
                my ($p, $others) = split(',', $1, 2);
                $p = "noname" unless(length($p));
                if(exists($tags{$p})){
                    $newct = $tags{$p} + length($3);
                    $tags{$p} = $newct;
                    $tagsplanned{$p} = $tagsplanned{$p} + $2;
                }else{
                    $tags{$p} = length($3);
                    $tagsplanned{$p} = $2;
                }
                if(length($others)){
                    if(exists($tags{$1})){
                        $newct = $tags{$1} + length($3);
                        $tags{$1} = $newct;
                        $tagsplanned{$1} = $tagsplanned{$p} + $2;
                    }else{
                        $tags{$1} = length($3);
                        $tagsplanned{$1} = $2;
                    }
                }
            }
        }
    }
}

printf "\t%15s %15s %17s %12s\n", "Earned Points","Planned Pomos","Completed Pomos","Efficiency";
if($summary){
    foreach $today (@days){
        print "$today:  ";
        &print_day_stats($today);
    }
    printf "\nTotals: \t%2s %15s %16s\t%11.02f\n",$tpoints,$tplan,$tpomos,$tpomos/$texp;
}else{
    print "$today:  ";
    &print_day_stats($today);
    print "\n";
}

if($itemize)
{
    printf "\nPrimary Project Worked:\n";
    foreach my $key (sort {$tags{$b} <=> $tags{$a} or $a cmp $b} keys %tags){
        printf("\n%30s: %3s/%s (%.02f%s)",$key,$tags{$key},$tpomos,$tags{$key}/$tpomos,'%') unless($key =~ /,/);
    }
    printf "\n\nSecondary Project Worked:\n";
    foreach my $key (sort {$tags{$b} <=> $tags{$a} or $a cmp $b} keys %tags){
        printf("\n%30s: %3s/%s (%.02f%s)",$key,$tags{$key},$tpomos,$tags{$key}/$tpomos,'%') if($key =~ /,/);
    }
    printf "\n\nPrimary Project Allocation:\n";
    foreach my $key (sort {$tagsplanned{$b} <=> $tagsplanned{$a} or $a cmp $b} keys %tagsplanned){
        printf("\n%30s: %3s/%s (%.02f%s)",$key,$tagsplanned{$key},$tplan,$tagsplanned{$key}/$tplan,'%') unless($key =~ /,/);
    }
}
# to sort values use keys to lookup the values and a compare block to compare
# them
print "\n"
