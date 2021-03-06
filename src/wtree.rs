/**
 * 权重树
 */

use std::sync::atomic::{AtomicUsize, Ordering as AOrd};
use std::sync::Arc;
use std::mem::replace;

#[derive(Clone)]
pub struct Item<T: Clone>{
    elem: T,
    count: usize, //自身权重值
    amount: usize, //自身权重值 和 子节点权重值的总和
    index: Arc<AtomicUsize>, //元素的位置
}

pub struct WeightTree<T: Clone>(Vec<Item<T>>);

impl<T: Clone> WeightTree<T> {

	//构建一颗权重树
	pub fn new() -> Self{
        WeightTree(Vec::new())
	}

	//创建一颗权重树， 并初始容量
	pub fn with_capacity(capacity: usize) -> Self{
		WeightTree(Vec::with_capacity(capacity))
	}

	//插入元素，返回该元素的位置
	pub fn push(&mut self, elem: T, weight: usize) -> Arc<AtomicUsize>{
		let len = self.0.len();
		self.0.push(Item{
			elem: elem,
			count: weight,
			amount: weight,
			index: Arc::new(AtomicUsize::new(len + 1)),
		});
		self.up(len)
	}

	//remove a element by index, Panics if index is out of bounds.
	pub fn remove(&mut self, index: Arc<AtomicUsize>) -> (T, usize){
		let i = index.load(AOrd::Relaxed);
		let r = self.delete((i - 1) as usize, self.0.len());
		(r.0, r.1)
	}

	//remove a element by index; returns it, or None if it is not exist;
	pub fn try_remove(&mut self, index: Arc<AtomicUsize>) -> Option<(T, usize)>{
		let i = index.load(AOrd::Relaxed);
		if i == 0{
			return None;
		}
		self.try_delete((i - 1) as usize)
	}

	//All element weights and
	pub fn amount(&mut self) -> usize{
		match self.0.len(){
			0 => 0,
			_ => self.0[0].amount
		}
	}

	//remove a element by weight and returns it, Panics if weight >= self.amount()
	pub fn remove_by_weight(&mut self, weight: usize) -> (T, usize){
		let index = self.find(weight, 0);
		let r = self.delete(index, self.0.len());
		(r.0, r.1)
	}

	//remove a element by weight, returns it, or None if weight >= self.amount()
	pub fn try_remove_by_weight(&mut self, weight: usize) -> Option<(T, usize)>{
		let len = self.0.len();
		match len{
			0 => None,
			_ => {
				let all_w = self.0[0].amount;
				match all_w <= weight{
					true => None,
					false => {
						let index = self.find(weight, 0);
						let r = self.delete(index, self.0.len());
						Some((r.0, r.1))
					}
				}
			}
		}
	}

	pub fn get(&self, index: usize) -> Option<&T>{
		match self.0.get(index){
			Some(v) => Some(&v.elem),
			None => {None}
		}
	}

	pub fn get_mut(&mut self, index: usize) -> Option<&mut T>{
		match self.0.get_mut(index){
			Some(v) => Some(&mut v.elem),
			None => None
		}
	}

	//get element by weight and returns its reference, Panics if weight >= self.amount()
	pub fn get_mut_by_weight(&mut self, weight: usize) -> (&mut T, Arc<AtomicUsize>){
		let index = self.find(weight, 0);
		let e = &mut self.0[index];
		(&mut e.elem, e.index.clone())
	}

	//get element by weight and returns its reference, or None if weight >= self.amount()
	pub fn try_get_mut_by_weight(&mut self, weight: usize) -> Option<(&mut T, Arc<AtomicUsize>)>{
		let len = self.0.len();
		match len{
			0 => None,
			_ => {
				let all_w = self.0[0].amount;
				match all_w <= weight{
					true => None,
					false => {
						let index = self.find(weight, 0);
						let e = &mut self.0[index];
						Some((&mut e.elem, e.index.clone()))
					}
				}
			}
		}
	}

	pub fn update_weight(&mut self, weight: usize, index: Arc<AtomicUsize>){
		let old_index = index.load(AOrd::Relaxed);
		let index = old_index - 1;
		let r_index = self.up_update(index, weight).load(AOrd::Relaxed);

		match r_index < old_index{
			true => (),
			false => self.down(index)
		}
	}
	
	pub fn len(&self) -> usize{
		self.0.len()
	}

	pub fn clear(&mut self) {
		self.0.clear();
	}

	//Finding element index according to weight
	#[inline]
	fn find(&mut self, mut weight: usize, cur_index:usize) -> usize{
		let cur_weight = self.0[cur_index].count;
		match weight < cur_weight{
			true => {//如果当前节点的权重比指定权重值大，应该直接返回该节点的索引
				return cur_index;
			},
			false => {//否则
				weight = weight - cur_weight;
				let left_index = (cur_index << 1) + 1;
				match self.0[left_index].amount <= weight{ //比较左节点及其所有子节点权重和与指定权重的大小
					true => weight = weight - self.0[left_index].amount, //如果指定权重更大， 则左节点及其所有子节点的权重都不可能超过指定权重， 从新计算指定权重， 在下一步从右节点中找节点
					false => return self.find(weight, left_index)//如果指定权重更小，则可以从左节点中找到需要的元素
				};
				return self.find(weight, left_index + 1);//从右节点中找
			}
		};
	}

	#[inline]
	fn delete(&mut self, index: usize, len: usize) -> (T, usize, Arc<AtomicUsize>){
		let mut elem = self.0.pop().unwrap();
		if index + 1 < len{//如果需要移除的元素不是堆底元素， 需要将该元素位置设置为栈底元素并下沉
			let de = &mut self.0[index];
			elem = replace(de, elem);
			de.amount = elem.amount - elem.count;
			self.down(index);
		}
		let mut cur = index;
		while cur > 0{
			cur = (cur - 1) >> 1;//parent
			self.0[cur].amount -= elem.count;
		}
		elem.index.store(0, AOrd::Relaxed);
		(elem.elem, elem.count, elem.index)
	}

	#[inline]
	fn try_delete(&mut self, index: usize) -> Option<(T, usize)>{
		let arr = &mut self.0;
		let len = arr.len();
		if index >= len {
			return None;
		}
		let r = self.delete(index, len);
		Some((r.0, r.1))
	}

	//上朔，更新当前节点和其父节点的权值  使用时应该保证index不会溢出
	fn up_update(&mut self, mut cur: usize, weight: usize) -> Arc<AtomicUsize>{
		let arr = &mut self.0;
		if cur > 0{
			let mut element = arr[cur].clone();
			let mut parent = (cur - 1) >> 1;
			let new_amount = element.amount + weight;
			while weight > arr[parent].count{
				let mut p = arr[parent].clone();
				p.index.store(cur + 1, AOrd::Relaxed);
				element.amount = p.amount + element.count;
				p.amount = new_amount - element.count + p.count;
				arr[cur] = p;
				
				// 往上迭代
				cur = parent;
				if parent == 0{
					break;
				}
				parent = (cur - 1) >> 1;
			}

			let mut i = cur;
			while i > 0{
				i = (i - 1) >> 1;//parent
				arr[i].amount = arr[i].amount - element.count + weight;
			}

			element.index.store(cur + 1, AOrd::Relaxed);
			arr[cur] = element;
		}
		let elem = &mut arr[cur];
		elem.amount = elem.amount - elem.count + weight;
		elem.count = weight;
		elem.index.clone()
	}

	//上朔， 使用时应该保证index不会溢出
	fn up(&mut self, mut cur: usize) -> Arc<AtomicUsize>{
		let arr = &mut self.0;
		if cur > 0{
			let mut element = arr[cur].clone();
			let mut parent = (cur - 1) >> 1;
			while element.count > arr[parent].count{
				let mut p = arr[parent].clone();
				p.index.store(cur + 1, AOrd::Relaxed);
				let ew = element.amount;
				element.amount = p.amount + element.count;
				p.amount = ew - element.count + p.count;
				arr[cur] = p;
				
				// 往上迭代
				cur = parent;
				if parent == 0{
					break;
				}
				parent = (cur - 1) >> 1;
			}

			let w = element.count;
			element.index.store(cur + 1, AOrd::Relaxed);
			arr[cur] = element;

			let mut i = cur;
			while i > 0{
				i = (i - 1) >> 1;//parent
				arr[i].amount += w;
			}
		}
		arr[cur].index.clone()
	}

	/**
	 * 下沉
	 * Panics if index is out of bounds.
	 */
	fn down(&mut self, index: usize) {
		let mut cur = index;
		let arr = &mut self.0;
		let mut element = arr[index].clone();
		let mut left = (cur << 1) + 1;
		let mut right = left + 1;
		let len = arr.len();

		while left < len {
			// 选择左右孩子的最较大值作为比较
			let mut child = left;
			if right < len && arr[right].count > arr[left].count {
				child = right;
			}
			
			match arr[index].count > arr[child].count{
				true => break,
				false => {
					let mut c = arr[child].clone();
					c.index.store(cur + 1, AOrd::Relaxed);
					let cw = c.amount;
					c.amount = element.amount;
					element.amount = cw - c.count + element.count;
					arr[cur] = c;
					
					// 往下迭代
					cur = child;
					left = (cur << 1) + 1;
					right = left + 1;
				}
			}
		}
		element.index.store(cur + 1, AOrd::Relaxed);
		arr[cur] = element;
	}
}

#[test]
fn test(){
	let mut wtree: WeightTree<u32> = WeightTree::new();
	wtree.push(100, 100);
	wtree.push(2000, 2000);
	wtree.push(50, 50);
	wtree.push(70, 70);
	wtree.push(500, 500);
	let index_2 = wtree.push(20, 20);
	assert_eq!(wtree.amount(), 2740);

	wtree.update_weight(60, index_2.clone());
	assert_eq!(index_2.load(AOrd::Relaxed), 3);
	assert_eq!(wtree.amount(), 2780);

	wtree.update_weight(20, index_2.clone());
	assert_eq!(wtree.amount(), 2740);
	assert_eq!(index_2.load(AOrd::Relaxed), 6);

	assert_eq!(wtree.remove_by_weight(2739).1, 20);
	assert_eq!(wtree.amount(), 2720);

	assert_eq!(wtree.remove_by_weight(2000).1, 500);
	assert_eq!(wtree.amount(), 2220);
	
	assert_eq!(wtree.remove_by_weight(1999).1, 2000);
	assert_eq!(wtree.amount(), 220);

	let index = wtree.push(30, 30);
	wtree.update_weight(80, index.clone());

	assert_eq!(wtree.remove_by_weight(140).1, 80);
	assert_eq!(wtree.amount(), 220);

}
