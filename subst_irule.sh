#!/bin/bash
# Script to substitute keywords/commands from irule files to make
# them pure tcl. The file will no longer produce the same result
# and will not be equivalent but it will run with tclscan.
if [ $# -ne 2 ]; then
	echo "iRule substitution script."
	echo "Usage: $0 <source> <destination>"
	exit 1
fi
source_filename="$1"
dest_filename="$2"

cp $source_filename $dest_filename 

subst='equals/eq,starts_with/=='

OLDIFS=$IFS; IFS=',';
for i in $subst;
do
	set -- $i; sed -i -e "s/$1/g" $dest_filename;
done;
IFS=$OLDIFS

sed -i '$ d' $dest_filename
sed -i '1,1d' $dest_filename
